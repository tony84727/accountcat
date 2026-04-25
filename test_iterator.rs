fn main() {
    let mut v = vec![1, 2, 3];
    let mut it = v.iter();
    let maybe_one = it.next();

    let to_try: Box<dyn Iterator<Item = &i32>> = if let Some(i) = maybe_one {
        Box::new(std::iter::once(i))
    } else {
        Box::new(v.iter())
    };
}
