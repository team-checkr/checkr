use serde::Serialize;

use crate::Tapi;

#[test]
fn basic_struct() {
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    struct A {
        a: i32,
        b: String,
    }

    insta::assert_display_snapshot!(<A as Tapi>::ts_decl().unwrap(), @"export type A = { a: number, b: string }");
}

#[test]
fn empty_struct() {
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    struct A {}

    insta::assert_display_snapshot!(<A as Tapi>::ts_decl().unwrap(), @"export type A = {  }");
}
#[test]
fn transparent_struct() {
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    #[serde(transparent)]
    struct A {
        x: Vec<i32>,
    }

    insta::assert_display_snapshot!(<A as Tapi>::ts_decl().unwrap(), @"export type A = number[];");
}

#[test]
fn basic_enum() {
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    enum A {
        X,
        Y,
        Z,
    }

    insta::assert_display_snapshot!(<A as Tapi>::ts_decl().unwrap(), @r###"
    export type A = "X" | "Y" | "Z";
    export const A: A[] = ["X", "Y", "Z"];
    "###);
}

#[test]
fn tagged_enum() {
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    #[serde(tag = "type")]
    enum A {
        X,
        Y,
        Z,
    }

    insta::assert_display_snapshot!(serde_json::to_string_pretty(&A::X).unwrap(), @r###"
    {
      "type": "X"
    }
    "###);

    insta::assert_display_snapshot!(<A as Tapi>::ts_decl().unwrap(), @r###"
    export type A = { "type": "X" } | { "type": "Y" } | { "type": "Z" };
    export const A: A[] = [{ "type": "X" }, { "type": "Y" }, { "type": "Z" }];
    "###);
}

#[test]
fn tagged_enum_with_data() {
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    #[serde(tag = "type")]
    enum A {
        X { wow: String },
        Y { thingy: String },
        Z,
    }

    let sample = [
        A::X {
            wow: "...".to_string(),
        },
        A::Y {
            thingy: "123".to_string(),
        },
        A::Z,
    ];
    insta::assert_display_snapshot!(serde_json::to_string_pretty(&sample).unwrap(), @r###"
    [
      {
        "type": "X",
        "wow": "..."
      },
      {
        "type": "Y",
        "thingy": "123"
      },
      {
        "type": "Z"
      }
    ]
    "###);

    insta::assert_display_snapshot!(<A as Tapi>::ts_decl().unwrap(), @r###"export type A = { "type": "X", "wow": string } | { "type": "Y", "thingy": string } | { "type": "Z" };"###);
}

#[test]
fn externally_tagged_with_data() {
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    enum A {
        X(String),
        Y { thingy: String },
        Z,
        W(i32, i32),
    }
    let sample = [
        A::X("...".to_string()),
        A::Y {
            thingy: "123".to_string(),
        },
        A::Z,
        A::W(1, 2),
    ];
    insta::assert_display_snapshot!(serde_json::to_string_pretty(&sample).unwrap(), @r###"
    [
      {
        "X": "..."
      },
      {
        "Y": {
          "thingy": "123"
        }
      },
      "Z",
      {
        "W": [
          1,
          2
        ]
      }
    ]
    "###);

    insta::assert_display_snapshot!(<A as Tapi>::ts_decl().unwrap(), @r###"export type A = { "X": string } | { "Y": { thingy: string } } | "Z" | { "W": [number, number] };"###);
}

#[test]
fn adjacent_with_data() {
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    #[serde(tag = "type", content = "data")]
    enum A {
        X(String),
        Y { thingy: String },
        Z,
        W(i32, i32),
    }

    let sample = [
        A::X("...".to_string()),
        A::Y {
            thingy: "123".to_string(),
        },
        A::Z,
        A::W(1, 2),
    ];
    insta::assert_display_snapshot!(serde_json::to_string_pretty(&sample).unwrap(), @r###"
    [
      {
        "type": "X",
        "data": "..."
      },
      {
        "type": "Y",
        "data": {
          "thingy": "123"
        }
      },
      {
        "type": "Z"
      },
      {
        "type": "W",
        "data": [
          1,
          2
        ]
      }
    ]
    "###);

    insta::assert_display_snapshot!(<A as Tapi>::ts_decl().unwrap(), @r###"export type A = { "type": "X", "data": string } | { "type": "Y", "data": { "thingy": string } } | { "type": "Z", "data": {  } } | { "type": "W", "data": [number, number] };"###);
}
