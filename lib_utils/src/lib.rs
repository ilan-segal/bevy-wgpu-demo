use itertools::Itertools;

pub fn cube_iter<I>(it: I) -> impl Iterator<Item = (I::Item, I::Item, I::Item)>
where
    I: Iterator + Clone,
    I::Item: Clone,
{
    iter_3d(it.clone(), it.clone(), it)
}

pub fn iter_3d<X, Y, Z>(x: X, y: Y, z: Z) -> impl Iterator<Item = (X::Item, Y::Item, Z::Item)>
where
    X: Iterator,
    Y: Iterator + Clone,
    Z: Iterator + Clone,
    X::Item: Clone,
    Y::Item: Clone,
    Z::Item: Clone,
{
    x.cartesian_product(y)
        .cartesian_product(z)
        .map(|((x, y), z)| (x, y, z))
}

pub fn square_iter<I>(it: I) -> impl Iterator<Item = (I::Item, I::Item)>
where
    I: Iterator + Clone,
    I::Item: Clone,
{
    it.clone().cartesian_product(it)
}
