use serde::Deserialize;

macro_rules! density_function {
    ($($name:ident ($alias:literal) $({ $($field:ident $(($field_alias:literal))?: $ty:ty),* })?;)*) => {
        #[derive(Deserialize, PartialEq, Debug, Clone)]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum DensityFunction {
            $(
                #[serde(alias = $alias)]
                $name $({
                    $(
                        $(#[serde(alias = $field_alias)])?
                        $field: $ty,
                    )*
                })?
            ),*
        }
    };
}

density_function!(
    Cache2d ("minecraft:cache_2d") { input ("argument"): DensityFunctionArgument };
    CacheAllInCell ("minecraft:cache_all_in_cell") { input ("argument"): DensityFunctionArgument };
    CacheOnce ("minecraft:cache_once") { input ("argument"): DensityFunctionArgument };
    FlatCache ("minecraft:flat_cache") { input ("argument"): DensityFunctionArgument };
    Interpolated ("minecraft:interpolated") { input ("argument"): DensityFunctionArgument };
    Abs ("minecraft:abs") { input ("argument"): DensityFunctionArgument };
    Cube ("minecraft:cube") { input ("argument"): DensityFunctionArgument };
    HalfNegative ("minecraft:half_negative") { input ("argument"): DensityFunctionArgument };
    IntervalSelect ("minecraft:interval_select") { input: DensityFunctionArgument, thresholds: Vec<f64>, functions: Vec<DensityFunctionArgument> };
    Invert ("minecraft:invert") { input ("argument"): DensityFunctionArgument };
    QuarterNegative ("minecraft:quarter_negative") { input ("argument"): DensityFunctionArgument };
    Square ("minecraft:square") { input ("argument"): DensityFunctionArgument };
    Squeeze ("minecraft:squeeze") { input ("argument"): DensityFunctionArgument };
    Add ("minecraft:add") { left ("argument1"): DensityFunctionArgument, right ("argument2"): DensityFunctionArgument };
    Div ("minecraft:div") { left ("argument1"): DensityFunctionArgument, right ("argument2"): DensityFunctionArgument };
    Mul ("minecraft:mul") { left ("argument1"): DensityFunctionArgument, right ("argument2"): DensityFunctionArgument };
    Sub ("minecraft:sub") { left ("argument1"): DensityFunctionArgument, right ("argument2"): DensityFunctionArgument };
    Min ("minecraft:min") { left ("argument1"): DensityFunctionArgument, right ("argument2"): DensityFunctionArgument };
    Max ("minecraft:max") { left ("argument1"): DensityFunctionArgument, right ("argument2"): DensityFunctionArgument };
    Beardifier ("minecraft:beardifier");
    BlendAlpha ("minecraft:blend_alpha");
    BlendDensity ("minecraft:blend_density") { input ("argument"): DensityFunctionArgument };
    BlendOffset ("minecraft:blend_offset");
    Ceil ("minecraft:ceil") { input: DensityFunctionArgument, multiple: i32 };
    Clamp ("minecraft:clamp") { input: DensityFunctionArgument, min: f64, max: f64 };
    Constant ("minecraft:constant") { value ("argument"): f64 };
    EndIslands ("minecraft:end_islands");
    FindTopSurface ("minecraft:find_top_surface") { density: DensityFunctionArgument, upper_bound: DensityFunctionArgument, lower_bound: i32, cell_height: i32 };
    Floor ("minecraft:floor") { input: DensityFunctionArgument, multiple: i32 };
    Lerp ("minecraft:lerp") { alpha: DensityFunctionArgument, first: DensityFunctionArgument, second: DensityFunctionArgument };
    Negate ("minecraft:negate") { input: DensityFunctionArgument };
    Noise ("minecraft:noise") { noise: String, xz_scale: f64, y_scale: f64 };
    OldBlendedNoise ("minecraft:old_blended_noise") { xz_scale: f64, y_scale: f64, xz_factor: f64, y_factor: f64, smear_scale_multiplier: f64 };
    RangeChoice ("minecraft:range_choice") { input: DensityFunctionArgument, min_inclusive: f64, max_exclusive: f64, when_in_range: DensityFunctionArgument, when_out_of_range: DensityFunctionArgument };
    Round ("minecraft:round") { input: DensityFunctionArgument, multiple: i32 };
    Shift ("minecraft:shift") { noise ("argument"): String };
    ShiftA ("minecraft:shift_a") { noise ("argument"): String };
    ShiftB ("minecraft:shift_b") { noise ("argument"): String };
    ShiftedNoise ("minecraft:shifted_noise") { noise: String, xz_scale: f64, y_scale: f64, shift_x: DensityFunctionArgument, shift_y: DensityFunctionArgument, shift_z: DensityFunctionArgument };
    Truncate ("minecraft:truncate") { input: DensityFunctionArgument, multiple: i32 };
    YClampedGradient ("minecraft:y_clamped_gradient") { from_y: i32, to_y: i32, from_value: f64, to_value: f64 };
    Spline ("minecraft:spline") { spline: DensitySpline };
    WeirdScaledSampler ("minecraft:weird_scaled_sampler") { rarity_value_mapper: String, noise: String, input: DensityFunctionArgument };
);

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct DensitySpline {
    pub coordinate: DensityFunctionArgument,
    pub points: Vec<DensitySplinePoint>,
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct DensitySplinePoint {
    pub location: f64,
    pub value: ValueOrSpline,
    pub derivative: f64,
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
#[serde(untagged)]
pub enum ValueOrSpline {
    Value(f64),
    Spline(DensitySpline),
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
#[serde(untagged)]
pub enum DensityFunctionArgument {
    Function(Box<DensityFunction>),
    Constant(f64),
    External(String),
}

pub fn deserialize_function(str: &str) -> serde_json::Result<DensityFunctionArgument> {
    serde_json::from_str(str)
}
