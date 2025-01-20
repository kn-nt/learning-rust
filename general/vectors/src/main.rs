fn main() {
    let a = vec![1, 2, 3, 4, 5];

    // With a start and an end
    println!("{:?}", &a[1..4]);

    // With a start and an end, inclusive
    println!("{:?}", &a[1..=3]);

    // With just a start
    println!("{:?}", &a[2..]);

    // With just an end
    println!("{:?}", &a[..3]);

    // With just an end, inclusive
    println!("{:?}", &a[..=2]);

    // All elements
    println!("{:?}", &a[..]);
}