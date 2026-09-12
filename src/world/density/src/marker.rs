use crate::DensityFunction;
use crate::wrapped::{CacheData, FlattenedDensityFunction, MarkerDensityFunction, WrapContext};

#[derive(Debug)]
pub struct CacheAllInCell(pub Box<dyn DensityFunction>);

#[derive(Debug)]
pub struct CacheOnce(pub Box<dyn DensityFunction>);

#[derive(Debug)]
pub struct Cache2d(pub Box<dyn DensityFunction>);

#[derive(Debug)]
pub struct FlatCache(pub Box<dyn DensityFunction>);

#[derive(Debug)]
pub struct Interpolated(pub Box<dyn DensityFunction>);

impl DensityFunction for CacheAllInCell {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        self.0.wrap(ctx)
    }
}

impl DensityFunction for CacheOnce {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        self.0.wrap(ctx)
    }
}

impl DensityFunction for Cache2d {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|ctx| FlattenedDensityFunction::Marker {
            op: MarkerDensityFunction::Cache2d(CacheData::default()),
            arg: self.0.wrap(ctx),
        })
    }
}

impl DensityFunction for FlatCache {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|ctx| FlattenedDensityFunction::Marker {
            op: MarkerDensityFunction::FlatCache(CacheData::default()),
            arg: self.0.wrap(ctx),
        })
    }
}

impl DensityFunction for Interpolated {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        self.0.wrap(ctx)
    }
}
