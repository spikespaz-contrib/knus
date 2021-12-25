use std::default::Default;

use proc_macro2::{TokenStream, Span};
use quote::quote;

use crate::definition::{Struct, StructBuilder, ArgKind, FieldAttrs, DecodeMode};
use crate::definition::{Child};

pub fn emit_struct(s: &Struct) -> syn::Result<TokenStream> {
    let s_name = &s.ident;
    let node = syn::Ident::new("node", Span::mixed_site());
    let children = syn::Ident::new("children", Span::mixed_site());
    let decode_args = decode_args(s, &node)?;
    let decode_props = decode_props(s, &node)?;
    let decode_children_normal = decode_children(s, &children,
                                          Some(quote!(#node.span())))?;
    let fields = s.all_fields();
    let mut extra_traits = Vec::new();
    let partial_compatible = !s.has_arguments && (
            s.properties.iter().all(|x| x.option || x.flatten) &&
            s.var_props.is_none()
        ) && (
            s.children.iter().all(|x| x.option || x.flatten) &&
            s.var_children.is_none()
        );
    if partial_compatible {
        let node = syn::Ident::new("node", Span::mixed_site());
        let name = syn::Ident::new("name", Span::mixed_site());
        let value = syn::Ident::new("value", Span::mixed_site());
        let insert_child = insert_child(s, &node)?;
        let insert_property = insert_property(s, &name, &value)?;
        extra_traits.push(quote! {
            impl<S> ::knuffel::traits::DecodePartial<S> for #s_name
                where S: ::knuffel::traits::Span,
            {
                fn insert_child(&mut self,
                    #node: &::knuffel::ast::SpannedNode<S>)
                    -> Result<bool, ::knuffel::Error<S>>
                {
                    #insert_child
                }
                fn insert_property(&mut self,
                    #name: &::knuffel::span::Spanned<Box<str>, S>,
                    #value: &::knuffel::ast::Value<S>)
                    -> Result<bool, ::knuffel::Error<S>>
                {
                    #insert_property
                }
            }
        });
    }
    if !s.has_arguments && !s.has_properties {
        let decode_children = decode_children(s, &children, None)?;
        extra_traits.push(quote! {
            impl<S> ::knuffel::traits::DecodeChildren<S> for #s_name
                where S: ::knuffel::traits::Span,
            {
                fn decode_children(#children: &[::knuffel::ast::SpannedNode<S>])
                    -> Result<Self, ::knuffel::Error<S>>
                {
                    #decode_children
                    Ok(#s_name {
                        #(#fields,)*
                    })
                }
            }
        });
    }
    Ok(quote! {
        #(#extra_traits)*
        impl<S: ::knuffel::traits::Span> ::knuffel::Decode<S> for #s_name {
            fn decode_node(#node: &::knuffel::ast::SpannedNode<S>)
                -> Result<Self, ::knuffel::Error<S>>
            {
                #decode_args
                #decode_props
                let #children = #node.children.as_ref()
                    .map(|lst| &lst[..]).unwrap_or(&[]);
                #decode_children_normal
                Ok(#s_name {
                    #(#fields,)*
                })
            }
        }
    })
}

fn decode_value(val: &syn::Ident, mode: &DecodeMode)
    -> syn::Result<TokenStream>
{
    match mode {
        DecodeMode::Normal => {
            Ok(quote!{
                ::knuffel::traits::DecodeScalar::decode(#val)
            })
        }
        DecodeMode::Str => {
            Ok(quote! {
                if let Some(typ) = &#val.type_name {
                    Err(::knuffel::Error::new(typ.span(),
                        format!("no type name expected")))
                } else {
                    match *#val.literal {
                        ::knuffel::ast::Literal::String(ref s) => {
                            ::std::str::FromStr::from_str(s)
                                .map_err(|e| ::knuffel::Error::from_err(
                                    #val.literal.span(), e))
                        }
                        _ => Err(::knuffel::Error::new(#val.literal.span(),
                                            "expected string value")),
                    }
                }
            })
        }
    }
}

fn decode_args(s: &Struct, node: &syn::Ident) -> syn::Result<TokenStream> {
    let mut decoder = Vec::new();
    let iter_args = syn::Ident::new("iter_args", Span::mixed_site());
    decoder.push(quote! {
        let mut #iter_args = #node.arguments.iter();
    });
    for arg in &s.arguments {
        let fld = &arg.field;
        let val = syn::Ident::new("val", Span::mixed_site());
        let decode_value = decode_value(&val, &arg.decode)?;
        match arg.kind {
            ArgKind::Value { option: true } => {
                decoder.push(quote! {
                    let #fld = #iter_args.next().map(|#val| {
                        #decode_value
                    }).transpose()?;
                });
            }
            ArgKind::Value { option: false } => {
                let error = format!("additional argument `{}` is required",
                                    fld);
                decoder.push(quote! {
                    let #val =
                        #iter_args.next().ok_or_else(|| {
                            ::knuffel::Error::new(
                                #node.node_name.span(), #error)
                        })?;
                    let #fld = #decode_value?;
                });
            }
        }
    }
    if let Some(var_args) = &s.var_args {
        let fld = &var_args.field;
        let val = syn::Ident::new("val", Span::mixed_site());
        let decode_value = decode_value(&val, &var_args.decode)?;
        decoder.push(quote! {
            let #fld = #iter_args.map(|#val| {
                #decode_value
            }).collect::<Result<_, _>>()?;
        });
    } else {
        decoder.push(quote! {
            if let Some(val) = #iter_args.next() {
                return Err(::knuffel::Error::new(val.literal.span(),
                                                 "unexpected argument"));
            }
        });
    }
    Ok(quote! { #(#decoder)* })
}

fn decode_props(s: &Struct, node: &syn::Ident) -> syn::Result<TokenStream> {
    let mut declare_empty = Vec::new();
    let mut match_branches = Vec::new();
    let mut postprocess = Vec::new();

    let val = syn::Ident::new("val", Span::mixed_site());
    let name = syn::Ident::new("name", Span::mixed_site());
    let name_str = syn::Ident::new("name_str", Span::mixed_site());

    for prop in &s.properties {
        let fld = &prop.field;
        let prop_name = prop.name();
        if prop.flatten {
            declare_empty.push(quote! {
                let mut #fld = ::std::default::Default::default();
            });
            match_branches.push(quote! {
                _ if ::knuffel::traits::DecodePartial::
                    insert_property(&mut #fld, #name, #val)?
                => {}
            });
        } else {
            let decode_value = decode_value(&val, &prop.decode)?;
            declare_empty.push(quote! {
                let mut #fld = None;
            });
            match_branches.push(quote! {
                #prop_name => #fld = Some(#decode_value?),
            });
            let req_msg = format!("property `{}` is required", prop_name);
            if !prop.option {
                postprocess.push(quote! {
                    let #fld = #fld.ok_or_else(|| {
                        ::knuffel::Error::new(#node.node_name.span(), #req_msg)
                    })?;
                });
            }
        }
    }
    if let Some(var_props) = &s.var_props {
        let fld = &var_props.field;
        let decode_value = decode_value(&val, &var_props.decode)?;
        declare_empty.push(quote! {
            let mut #fld = Vec::new();
        });
        match_branches.push(quote! {
            #name_str => {
                let converted_name = #name_str.parse()
                    .map_err(|e| ::knuffel::Error::from_err(#name.span(), e))?;
                #fld.push((
                    converted_name,
                    #decode_value?,
                ));
            }
        });
        postprocess.push(quote! {
            let #fld = #fld.into_iter().collect();
        });
    } else {
        match_branches.push(quote! {
            #name_str => {
                return Err(::knuffel::Error::new(#name.span(),
                    format!("unexpected property `{}`",
                            #name_str.escape_default())));
            }
        });
    };
    Ok(quote! {
        #(#declare_empty)*
        for (#name, #val) in #node.properties.iter() {
            match &***#name {
                #(#match_branches)*
            }
        }
        #(#postprocess)*
    })
}

fn unwrap_fn(func: &syn::Ident, name: &syn::Ident, attrs: &FieldAttrs)
    -> syn::Result<TokenStream>
{
    let mut bld = StructBuilder::new(
        syn::Ident::new(&format!("Wrap_{}", name), Span::mixed_site()),
        Default::default(),
    );
    bld.add_field(name.clone(), false, attrs)?;
    let s = bld.build();

    let node = syn::Ident::new("node", Span::mixed_site());
    let children = syn::Ident::new("children", Span::mixed_site());
    let decode_args = decode_args(&s, &node)?;
    let decode_props = decode_props(&s, &node)?;
    let decode_children = decode_children(&s, &children,
                                          Some(quote!(#node.span())))?;
    Ok(quote! {
        let #func = |#node: &::knuffel::ast::SpannedNode<S>| {
            #decode_args
            #decode_props
            let #children = #node.children.as_ref()
                .map(|lst| &lst[..]).unwrap_or(&[]);
            #decode_children

            Ok(#name)
        };
    })
}

fn decode_node(child_def: &Child, in_partial: bool, child: &syn::Ident)
    -> syn::Result<TokenStream>
{
    let fld = &child_def.field;
    let dest = if in_partial {
        quote!(self.#fld)
    } else {
        quote!(#fld)
    };
    if let Some(inner) = child_def.unwrap.as_ref() {
        let func = syn::Ident::new(&format!("unwrap_{}", fld),
                                   Span::mixed_site());
        let unwrap_fn = unwrap_fn(&func, fld, inner)?;
        if in_partial {
            Ok(quote! {
                #unwrap_fn
                #dest = Some(#func(#child)?);
                Ok(true)
            })
        } else {
            Ok(quote! {
                #unwrap_fn
                match #func(#child) {
                    Ok(#child) => {
                        #dest = Some(#child);
                        None
                    }
                    Err(e) => Some(Err(e)),
                }
            })
        }
    } else {
        if in_partial {
            Ok(quote! {
                #dest = Some(::knuffel::Decode::decode_node(#child)?);
                Ok(true)
            })
        } else {
            Ok(quote! {
                match ::knuffel::Decode::decode_node(#child) {
                    Ok(#child) => {
                        #dest = Some(#child);
                        None
                    }
                    Err(e) => Some(Err(e)),
                }
            })
        }
    }
}

fn insert_child(s: &Struct, node: &syn::Ident) -> syn::Result<TokenStream> {
    let mut match_branches = Vec::with_capacity(s.children.len());
    for child_def in &s.children {
        let fld = &child_def.field;
        let child_name = &child_def.name;
        if child_def.flatten {
            match_branches.push(quote! {
                _ if ::knuffel::traits::DecodePartial
                    ::insert_child(&mut self.#fld, #node)?
                => Ok(true),
            })
        } else {
            let dup_err = format!("duplicate node `{}`, single node expected",
                                  child_name.escape_default());
            let decode = decode_node(&child_def, true, node)?;
            match_branches.push(quote! {
                #child_name => {
                    if self.#fld.is_some() {
                        Err(::knuffel::Error::new(#node.node_name.span(), #dup_err))
                    } else {
                        #decode
                    }
                }
            });
        }
    }
    Ok(quote! {
        match &**#node.node_name {
            #(#match_branches)*
            _ => Ok(false),
        }
    })
}

fn insert_property(s: &Struct, name: &syn::Ident, value: &syn::Ident)
    -> syn::Result<TokenStream>
{
    let mut match_branches = Vec::with_capacity(s.children.len());
    for prop in &s.properties {
        let fld = &prop.field;
        let prop_name = prop.name();
        if prop.flatten {
            match_branches.push(quote! {
                _ if ::knuffel::traits::DecodePartial
                    ::insert_property(&mut self.#fld, #name, #value)?
                => Ok(true),
            });
        } else {
            let decode_value = decode_value(&value, &prop.decode)?;
            match_branches.push(quote! {
                #prop_name => {
                    self.#fld = Some(#decode_value?);
                    Ok(true)
                }
            });
        }
    }
    Ok(quote! {
        match &***#name {
            #(#match_branches)*
            _ => Ok(false),
        }
    })
}

fn decode_children(s: &Struct, children: &syn::Ident,
                   err_span: Option<TokenStream>)
    -> syn::Result<TokenStream>
{
    let mut declare_empty = Vec::new();
    let mut match_branches = Vec::new();
    let mut postprocess = Vec::new();

    let child = syn::Ident::new("child", Span::mixed_site());
    let name_str = syn::Ident::new("name_str", Span::mixed_site());
    for child_def in &s.children {
        let fld = &child_def.field;
        let child_name = &child_def.name;
        if child_def.flatten {
            declare_empty.push(quote! {
                let mut #fld = ::std::default::Default::default();
            });
            match_branches.push(quote! {
                _ if (
                    match ::knuffel::traits::DecodePartial
                        ::insert_child(&mut #fld, #child)
                    {
                        Ok(true) => return None,
                        Ok(false) => false,
                        Err(e) => return Some(Err(e)),
                    }
                ) => None,
            })
        } else {
            declare_empty.push(quote! {
                let mut #fld = None;
            });
            let dup_err = format!("duplicate node `{}`, single node expected",
                                  child_name.escape_default());
            let decode = decode_node(&child_def, false, &child)?;
            match_branches.push(quote! {
                #child_name => {
                    if #fld.is_some() {
                        Some(Err(::knuffel::Error::new(#child.node_name.span(),
                                                       #dup_err)))
                    } else {
                        #decode
                    }
                }
            });
            let req_msg = format!("child node `{}` is required", child_name);
            if !child_def.option {
                if let Some(span) = &err_span {
                    postprocess.push(quote! {
                        let #fld = #fld.ok_or_else(|| {
                            ::knuffel::Error::new(#span, #req_msg)
                        })?;
                    });
                } else {
                    postprocess.push(quote! {
                        let #fld = #fld.ok_or_else(|| {
                            ::knuffel::Error::new_global(#req_msg)
                        })?;
                    });
                }
            }
        }
    }
    if let Some(var_children) = &s.var_children {
        let fld = &var_children.field;
        match_branches.push(quote! {
            _ => {
                match ::knuffel::Decode::decode_node(#child) {
                    Ok(#child) => Some(Ok(#child)),
                    Err(e) => Some(Err(e)),
                }
            }
        });
        Ok(quote! {
            #(#declare_empty)*
            let #fld = #children.iter().flat_map(|#child| {
                match &**#child.node_name {
                    #(#match_branches)*
                }
            }).collect::<Result<_, ::knuffel::Error<_>>>()?;
            #(#postprocess)*
        })
    } else {
        match_branches.push(quote! {
            #name_str => {
                Some(Err(::knuffel::Error::new(#child.span(),
                    format!("unexpected node `{}`",
                            #name_str.escape_default()))))
            }
        });

        Ok(quote! {
            #(#declare_empty)*
            #children.iter().flat_map(|#child| {
                match &**#child.node_name {
                    #(#match_branches)*
                }
            }).collect::<Result<(), ::knuffel::Error<_>>>()?;
            #(#postprocess)*
        })
    }
}
