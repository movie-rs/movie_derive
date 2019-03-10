extern crate proc_macro;
use proc_macro::TokenStream;

use std::collections::HashMap;

#[proc_macro]
pub fn actor(input: TokenStream) -> TokenStream {
    actor_internal(input, false)
}
#[proc_macro]
pub fn actor_dbg(input: TokenStream) -> TokenStream {
    actor_internal(input, true)
}

// Input: "SimplestActor input : Ping , on_message : Ping => Pong ,"
fn actor_internal(input: TokenStream, debug: bool) -> TokenStream {
    let input = input.to_string();

    // Locate attributes inside input string
    // (start, name, start_without_name)
    let mut locations = vec![(0, "name", 0)];
    let mut try_find = |attr| {
        // Any of the following may happen:
        // - "\ninput :\n"
        // - " input :\n"
        // - "\ninput : "
        // - " input : "
        let search_str = format!("{} :", attr);
        let pos = input.find(&search_str);
        if let Some(pos) = pos {
            locations.push((pos, attr, pos + search_str.len()));
            return;
        }
    };
    try_find("input");
    try_find("input_derive");
    try_find("data");
    try_find("on_init");
    try_find("on_message");
    locations.sort_unstable();

    // attrs = {
    //     "input": "Ping ,"
    //     "name": "SimplestActor",
    //     "on_message": "Ping => Pong ,",
    // }
    let mut attrs: HashMap<&str, String> = HashMap::new();

    // (start, name, start_without_name)
    for i in 0..locations.len() {
        let value = if i == locations.len() - 1 {
            &input[locations[i].2..] // Last segment
        } else {
            &input[locations[i].2..locations[i + 1].0] // Start of next segment means this one ends
        };
        attrs.insert(locations[i].1, value.to_string());
    }

    if debug {
        dbg!(&attrs);
    }

    // Check for missing required attrs
    if !attrs.contains_key("input") {
        panic!("Actor must accept some input (consider accepting Tick or Stop) - define enum types in `input`");
    }
    if !attrs.contains_key("on_message") {
        panic!("Actor must have on_message handler");
    }

    // Check for presence of attributes that change code flow
    let has_data = attrs.contains_key("data");
    // TODO: "()" turns into "(  )". This may change in the future, so a better
    // way to determine this should be implemented
    let accepts_tick =
        attrs["input"].find(", Tick , ").is_some() || attrs["input"].find(", Tick ,\n").is_some();

    // Assign default values for missing optional supported attrs
    attrs.entry("data").or_insert("".to_string());
    attrs.entry("on_init").or_insert("".to_string());

    // Prepare strings used later
    let input_derive = if attrs.contains_key("input_derive") {
        format!("#[derive({})]", attrs["input_derive"])
    } else {
        "".to_string()
    };

    // TODO: Consider rewriting to quote!()
    let output = format!(
        "
        mod {name} {{
        pub struct Actor {{
            running: bool,
            data: Data,
        }}
        pub struct Data {{
            {data}
        }}
        {input_derive}
        pub enum Input {{
            {input}
        }}
        impl Actor {{
            pub fn start({optional_data_argument}) -> movie::Handle<
                std::thread::JoinHandle<()>,
                Input,
                >
            {{
                let (tx_ota, rx_ota) = std::sync::mpsc::channel(); // owner-to-actor
                let handle = std::thread::spawn(move || {{
                    {{
                        // newline in case on_init ends with a comment
                        {on_init}
                    }};
                    {optional_default_data}
                    let mut actor = Actor {{
                        running: true,
                        data,
                    }};
                    while actor.running {{
                        let mut on_message = |message: Input| {{
                            use Input::*;
                            match message {{
                                {on_message}
                            }};
                        }};
                        while let Ok(message) = rx_ota.try_recv() {{
                            on_message(message);
                        }}
                        {optional_tick_handler};
                        // sleep for 4 ms before polling or ticking
                        // 4 ms is minimum on some Linux systems
                        // so it was chosen for compatibility
                        use std::thread::sleep;
                        use std::time::Duration;
                        sleep(Duration::from_millis(4));
                    }}
                }});
                movie::Handle {{
                    join_handle: handle,
                    tx: tx_ota,
                }}
            }}
        }}
        }}",
        // attrs
        name = attrs["name"],
        input = attrs["input"],
        data = attrs["data"],
        on_init = attrs["on_init"],
        on_message = attrs["on_message"],
        // prepared strings
        input_derive = input_derive,
        // conditional code
        optional_tick_handler = if accepts_tick {
            "on_message(Input::Tick)"
        } else {
            ""
        },
        optional_data_argument = if has_data { "data: Data" } else { "" },
        optional_default_data = if has_data { "" } else { "let data = Data {};" },
    );
    if debug {
        println!("Generated code:");
        println!("{}", output);
    }
    output.parse().unwrap()
}
