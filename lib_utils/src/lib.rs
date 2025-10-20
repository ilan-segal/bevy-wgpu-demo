use itertools::Itertools;

pub fn cube_iter<I>(it: I) -> impl Iterator<Item = (I::Item, I::Item, I::Item)>
where
    I: Iterator + Clone,
    I::Item: Clone,
{
    it.clone()
        .cartesian_product(it.clone())
        .cartesian_product(it.clone())
        .map(|((x, y), z)| (x, y, z))
}

pub fn square_iter<I>(it: I) -> impl Iterator<Item = (I::Item, I::Item)>
where
    I: Iterator + Clone,
    I::Item: Clone,
{
    it.clone().cartesian_product(it)
}
