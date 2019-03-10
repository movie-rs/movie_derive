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
        // Any of the following cases may happen:
        let search_strings = &[
            format!("\n{} :\n", attr),
            format!(" {} :\n", attr),
            format!("\n{} : ", attr),
            format!(" {} : ", attr),
            format!("\n{}\n:\n", attr),
            format!(" {}\n:\n", attr),
            format!("\n{}\n: ", attr),
            format!(" {}\n: ", attr),
        ];
        for search_str in search_strings {
            let pos = input.find(search_str);
            if let Some(pos) = pos {
                locations.push((pos, attr, pos + search_str.len()));
                return;
            }
        }
    };
    try_find("input");
    try_find("input_derive");
    try_find("data");
    try_find("on_init");
    try_find("on_message");
    try_find("tick_rate");
    try_find("on_tick");
    try_find("on_stop");
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

    // Assign default values for missing optional supported attrs
    attrs.entry("data").or_insert("".to_string());
    attrs.entry("on_init").or_insert("".to_string());
    attrs.entry("tick_rate").or_insert("10".to_string());
    attrs.entry("on_tick").or_insert("".to_string());
    attrs.entry("on_stop").or_insert("".to_string());

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
            {data}
        }}
        {input_derive}
        pub enum Input {{
            {input}
        }}
        impl Actor {{
            pub fn start(mut self) -> movie::Handle<
                std::thread::JoinHandle<()>,
                Input,
                >
            {{
                let (tx_ota, rx_ota) = std::sync::mpsc::channel(); // owner-to-actor data
                let (tx_kill, rx_kill) = std::sync::mpsc::channel(); // owner-to-actor stop requests
                let handle = std::thread::spawn(move || {{
                    {{
                        // newline in case on_init ends with a comment
                        {on_init}
                    }};
                    let mut running = true;
                    while running {{
                        let mut on_message = |message: Input| {{
                            use Input::*;
                            match message {{
                                {on_message}
                            }};
                        }};
                        while let Ok(message) = rx_ota.try_recv() {{
                            on_message(message);
                        }}
                        if let Ok(_) = rx_kill.try_recv() {{
                            {{
                                {on_stop}
                            }};
                            running = false;
                        }}
                        {{
                            {on_tick}
                        }};
                        // sleep for 4 ms before polling or ticking
                        // 4 ms is minimum on some Linux systems
                        // so it was chosen for compatibility
                        use std::thread::sleep;
                        use std::time::Duration;
                        sleep(Duration::from_millis(1000 / {tick_rate}));
                    }}
                }});
                movie::Handle {{
                    join_handle: handle,
                    tx: tx_ota,
                    kill: tx_kill,
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
        tick_rate = attrs["tick_rate"],
        on_tick = attrs["on_tick"],
        on_stop = attrs["on_stop"],
        // prepared strings
        input_derive = input_derive,
    );
    if debug {
        println!("Generated code:");
        println!("{}", output);
    }
    output.parse().unwrap()
}
