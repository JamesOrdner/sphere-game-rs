// Variants lower on the list will have higher state priority
#[derive(PartialEq, PartialOrd)]
pub enum SubsystemType {
    Camera,
    Input,
    Physics,
}
