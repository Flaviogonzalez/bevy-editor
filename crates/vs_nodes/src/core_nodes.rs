//! The hand-written nodes: arithmetic, logic, text, and building and taking apart vectors,
//! rotations and colors. Methods of Bevy's math types are in the generated `glam` set.

nodes! {
    core;
    "Math" math {
        /// a + b
        "Add" fn add(a: f32, b: f32) -> f32 { a + b }
        /// a - b
        "Subtract" fn sub(a: f32, b: f32) -> f32 { a - b }
        /// a × b
        "Multiply" fn mul(a: f32, b: f32) -> f32 { a * b }
        /// a ÷ b. Dividing by zero gives infinity, or NaN for 0 ÷ 0.
        "Divide" fn div(a: f32, b: f32) -> f32 { a / b }
        /// The smaller of a and b.
        "Min" fn min(a: f32, b: f32) -> f32 { a.min(b) }
        /// The larger of a and b.
        "Max" fn max(a: f32, b: f32) -> f32 { a.max(b) }
        /// Sine of an angle in radians.
        "Sin" fn sin(x: f32) -> f32 { x.sin() }
        /// Cosine of an angle in radians.
        "Cos" fn cos(x: f32) -> f32 { x.cos() }
        /// Absolute value.
        "Abs" fn abs(x: f32) -> f32 { x.abs() }
        /// The angle of the point (x, y) from the X axis, in radians.
        "Atan2" use atan2(y: f32, x: f32) -> f32 = f32::atan2;
        /// Keeps a number between min and max. If min is above max, max wins. (Not `f32::clamp`,
        /// which panics then: no node may panic.)
        "Clamp" fn clamp(value: f32, min: f32, max: f32) -> f32 { value.max(min).min(max) }
        /// Interpolates between a and b: t = 0 gives a, t = 1 gives b.
        "Lerp" fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
        /// A constant number.
        "Number" fn number(value: f32) -> f32 { value }
    }
    "Logic" logic {
        /// a > b
        "Greater" fn gt(a: f32, b: f32) -> bool { a > b }
        /// a < b
        "Less" fn lt(a: f32, b: f32) -> bool { a < b }
        /// a == b, compared exactly.
        "Equal" fn eq(a: f32, b: f32) -> bool { a == b }
        /// True when both are true.
        "And" fn and(a: bool, b: bool) -> bool { a && b }
        /// True when either is true.
        "Or" fn or(a: bool, b: bool) -> bool { a || b }
        /// Inverts a boolean.
        "Not" fn not(value: bool) -> bool { !value }
    }
    "Text" text {
        /// Concatenates two texts.
        "Join Text" fn join(a: String, b: String) -> String { a + &b }
        /// A number as text, formatted the way Rust formats an `f32`.
        "Number to Text" fn from_number(value: f32) -> String { value.to_string() }
        /// `true` or `false`.
        "Bool to Text" fn from_bool(value: bool) -> String { value.to_string() }
        /// A vector as text, such as `[1, 2, 3]`.
        "Vector to Text" fn from_vec3(value: Vec3) -> String { value.to_string() }
        /// A rotation as text, as its quaternion.
        "Rotation to Text" fn from_quat(value: Quat) -> String { value.to_string() }
        /// A color as text.
        "Color to Text" fn from_color(value: Color) -> String { format!("{:?}", value) }
    }
    "Vector" vector {
        /// A vector from its three components.
        "Make Vector" fn new(x: f32, y: f32, z: f32) -> Vec3 { Vec3::new(x, y, z) }
        /// The same number in all three components.
        "Splat" fn splat(value: f32) -> Vec3 { Vec3::splat(value) }
        /// The vector's X component.
        "Vector X" fn x(v: Vec3) -> f32 { v.x }
        /// The vector's Y component.
        "Vector Y" fn y(v: Vec3) -> f32 { v.y }
        /// The vector's Z component.
        "Vector Z" fn z(v: Vec3) -> f32 { v.z }
        /// a + b
        "Add Vectors" fn add(a: Vec3, b: Vec3) -> Vec3 { a + b }
        /// a - b
        "Subtract Vectors" fn sub(a: Vec3, b: Vec3) -> Vec3 { a - b }
        /// Multiplies every component by a number.
        "Scale Vector" fn scale(v: Vec3, factor: f32) -> Vec3 { v * factor }
        /// Points the other way.
        "Negate Vector" fn neg(v: Vec3) -> Vec3 { -v }
    }
    "Rotation" rotation {
        /// A rotation from Euler angles in degrees, applied X, then Y, then Z.
        "Rotation from Euler" fn from_euler(x: f32, y: f32, z: f32) -> Quat {
            Quat::from_euler(EulerRot::XYZ, x.to_radians(), y.to_radians(), z.to_radians())
        }
        /// A rotation of some degrees around an axis.
        "Rotation around Axis" fn from_axis_angle(axis: Vec3, degrees: f32) -> Quat {
            Quat::from_axis_angle(axis.normalize_or_zero(), degrees.to_radians())
        }
        /// Applies b, then a.
        "Combine Rotations" fn mul(a: Quat, b: Quat) -> Quat { a * b }
        /// Rotates a vector.
        "Rotate Vector" fn rotate(rotation: Quat, v: Vec3) -> Vec3 { rotation * v }
    }
    "Color" color {
        /// A color from red, green, blue and alpha, each from 0 to 1, in sRGB.
        "Make Color" fn srgba(red: f32, green: f32, blue: f32, alpha: f32) -> Color { Color::srgba(red, green, blue, alpha) }
        /// Mixes two colors: t = 0 gives a, t = 1 gives b.
        "Mix Colors" fn mix(a: Color, b: Color, t: f32) -> Color { a.mix(&b, t) }
        /// The same color with another alpha.
        "With Alpha" fn with_alpha(color: Color, alpha: f32) -> Color { color.with_alpha(alpha) }
        /// The color's red, from 0 to 1, in sRGB.
        "Color Red" fn red(color: Color) -> f32 { color.to_srgba().red }
        /// The color's green, from 0 to 1, in sRGB.
        "Color Green" fn green(color: Color) -> f32 { color.to_srgba().green }
        /// The color's blue, from 0 to 1, in sRGB.
        "Color Blue" fn blue(color: Color) -> f32 { color.to_srgba().blue }
        /// The color's alpha, from 0 to 1.
        "Color Alpha" fn alpha(color: Color) -> f32 { color.alpha() }
    }
}
