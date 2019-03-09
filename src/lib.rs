extern crate proc_macro;
use proc_macro::TokenStream;

use std::collections::HashMap;

#[proc_macro]
// input: "SimplestActor gets : Ping , sends : Pong , on_message : Ping => Pong ,"
pub fn actor(input: TokenStream) -> TokenStream {
    // parse input
    let input = input.to_string();
    let mut attrs: HashMap<&str, &str> = HashMap::new();

    let mut positions = vec![(0, "name")];
    let try_find = |attr| {
        let pos = input.find(&format!(" {} : ", attr));
    };
    positions.push();

    let config = "";
    let data = "";
    format!(
        // const NAME: &'static str = \"{name}\";
        // struct {name} {{
        //     {config},
        //     {data},
        // }}
    "",
        // name = name,
        // config = config,
        // data = data
    )
    .parse()
    .unwrap()
}
