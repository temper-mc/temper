use crate::wrapped::WrappedDensityFunction;
use temper_core::math::TemperMathExt;

pub enum FlattenedSpline<'a> {
    Multipoint {
        coordinate: usize,
        locations: &'a [f64],
        derivatives: &'a [f64],
        values: Vec<FlattenedSpline<'a>>,
    },
    Constant(f64),
}

impl FlattenedSpline<'_> {
    pub fn sample(&mut self, func: &WrappedDensityFunction) -> f64 {
        match self {
            FlattenedSpline::Multipoint {
                coordinate,
                locations,
                derivatives,
                values,
            } => {
                let input = func.execute_inner(*coordinate);
                let last_index = locations.len() - 1;

                match Self::find_interval_start(locations, input) {
                    None => {
                        let value = values[0].sample(func);
                        Self::linear_extend(input, locations, derivatives, value, 0)
                    }
                    Some(x) if x == last_index => {
                        let value = values[last_index].sample(func);
                        Self::linear_extend(input, locations, derivatives, value, last_index)
                    }
                    Some(start) => {
                        let x1 = locations[start];
                        let x2 = locations[start + 1];
                        let t = input.inverse_lerp(x1, x2);
                        let y1 = values[start].sample(func);
                        let y2 = values[start + 1].sample(func);
                        let d1 = derivatives[start];
                        let d2 = derivatives[start + 1];
                        let a = d1 * (x2 - x1) - (y2 - y1);
                        let b = -d2 * (x2 - x1) + (y2 - y1);
                        t.lerp(y1, y2) + t * (1.0 - t) * t.lerp(a, b)
                    }
                }
            }
            FlattenedSpline::Constant(c) => *c,
        }
    }

    fn linear_extend(
        input: f64,
        locations: &[f64],
        derivatives: &[f64],
        value: f64,
        index: usize,
    ) -> f64 {
        let derivative = derivatives[index];

        if derivative == 0.0 {
            value
        } else {
            value + derivative * (input - locations[index])
        }
    }

    fn find_interval_start(locations: &[f64], input: f64) -> Option<usize> {
        locations
            .partition_point(|&location| input >= location)
            .checked_sub(1)
    }
}
