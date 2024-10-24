Currently `DecodeScalar` derive is only implemented for enums

# Enums

Only enums that contain no data are supported:
```rust
#[derive(knus::DecodeScalar)]
enum Color {
    Red,
    Blue,
    Green,
    InfraRed,
}
```

This will match scalar values in `kebab-case`. For example, this node decoder:
```
# #[derive(knus::DecodeScalar)]
# enum Color { Red, Blue, Green, InfraRed }
#[derive(knus::Decode)]
struct Document {
    #[knus(child, unwrap(arguments))]
    all_colors: Vec<Color>,
}
```

Can be populated from the following text:
```kdl
all-colors "red" "blue" "green" "infra-red"
```
