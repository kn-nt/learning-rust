use std::cmp::PartialEq;

#[derive(Clone)]
struct TestStruct {
    a: i32,
}
#[derive(Clone)]
struct TestStruct2 {
    a: i32,
}

#[derive(Debug)]
struct StringTest {
    a: String,
    b: String,
}

struct StructPair<'a> {
    TestStruct: &'a TestStruct,
    TestStruct2: &'a TestStruct2,
}

impl PartialEq for StructPair<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.TestStruct.a == other.TestStruct.a &&
            self.TestStruct2.a == other.TestStruct2.a
    }
}

fn main() {
    println!("Hello, world!");
    let t1 = TestStruct { a: 1 };
    let t2 = TestStruct2 { a: 1 };
    let t3 = TestStruct { a: 1 };
    let t4 = TestStruct2 { a: 2 };

    let s1 = StructPair {
        TestStruct: &t1,
        TestStruct2: &t2,
    };
    let s2 = StructPair {
        TestStruct: &t3,
        TestStruct2: &t4,
    };

    if s1 == s2 {
        println!("s1 == s2");
    } else {
        println!("s1 != s2");
    }

    let st = StringTest { a: "myTest".to_owned(), b: "wow".to_owned() };

    let new_owner = st.a;

    println!("New owner is {}", new_owner);
    println!("{:?}", st.b);
}
