use crate::wrapped::spline::FlattenedSpline;
use crate::wrapped::{FlattenedDensityFunction, WrapContext};
use crate::{BoxedDensityFunction, DensityFunction};

#[derive(Debug)]
pub enum Spline {
    Multipoint {
        coordinate: BoxedDensityFunction,
        locations: Vec<f64>,
        values: Vec<Spline>,
        derivatives: Vec<f64>,
    },
    Constant {
        value: f64,
    },
}

impl Spline {
    fn wrap_type<'a>(&'a self, ctx: &mut WrapContext<'a>) -> FlattenedSpline<'a> {
        match self {
            Spline::Multipoint {
                values,
                locations,
                derivatives,
                coordinate,
            } => FlattenedSpline::Multipoint {
                locations,
                derivatives,
                coordinate: coordinate.wrap(ctx),
                values: values.iter().map(|v| v.wrap_type(ctx)).collect(),
            },
            Spline::Constant { value } => FlattenedSpline::Constant(*value),
        }
    }
}

impl DensityFunction for Spline {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        match self {
            Spline::Constant { value } => {
                ctx.push_op(|_| FlattenedDensityFunction::Constant(*value))
            }
            Spline::Multipoint {
                coordinate,
                locations,
                values,
                derivatives,
            } => ctx.push_op(|ops| {
                FlattenedDensityFunction::Spline(FlattenedSpline::Multipoint {
                    coordinate: coordinate.wrap(ops),
                    locations,
                    derivatives,
                    values: values.iter().map(|s| s.wrap_type(ops)).collect(),
                })
            }),
        }
    }
}
