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

// input: "SimplestActor gets : Ping , sends : Pong , on_message : Ping => Pong ,"
fn actor_internal(input: TokenStream, debug: bool) -> TokenStream {
    let input = input.to_string();

    // locate attributes inside input string
    // (start, name, start_without_name)
    let mut locations = vec![(0, "name", 0)];
    let mut try_find = |attr| {
        let search_str = format!(" {} : ", attr);
        let pos = input.find(&search_str);
        if let Some(pos) = pos {
            locations.push((pos, attr, pos + search_str.len()));
        } else {
            // TODO: figure out why in StreamParsingActor example
            // spaces are replaced by newlines
            let search_str = format!(" {} :\n", attr);
            let pos = input.find(&search_str);
            if let Some(pos) = pos {
                locations.push((pos, attr, pos + search_str.len()));
            }
        }
    };
    try_find("gets");
    try_find("sends");
    try_find("data");
    try_find("on_init");
    try_find("on_message");
    locations.sort_unstable();

    // attrs = {
    //     "name": "SimplestActor",
    //     "sends": "Pong ,",
    //     "on_message": "Ping => Pong ,",
    //     "gets": "Ping ,"
    // }
    let mut attrs: HashMap<&str, String> = HashMap::new();

    // (start, name, start_without_name)
    for i in 0..locations.len() {
        let value = if i == locations.len() - 1 {
            &input[locations[i].2..] // last segment
        } else {
            &input[locations[i].2..locations[i + 1].0] // start of next segment means this one ends
        };
        attrs.insert(locations[i].1, value.to_string());
    }

    if debug {
        dbg!(&attrs);
    }

    // check for missing required attrs
    if !attrs.contains_key("gets") {
        panic!("Actor must accept some input (consider accepting Tick or Stop) - define enum types in `gets`");
    }
    if !attrs.contains_key("sends") {
        panic!("Actor must send some message (consider sending Ok) - define enum types in `sends`");
    }
    if !attrs.contains_key("on_message") {
        panic!("Actor must have on_message handler");
    }

    let has_data = attrs.contains_key("data");
    let tick =
        attrs["gets"].find(", Tick , ").is_some() || attrs["gets"].find(", Tick ,\n").is_some();

    // assign default values for missing optional supported attrs
    attrs.entry("data").or_insert("".to_string());
    attrs.entry("on_init").or_insert("".to_string());

    // TODO: consider rewriting to quote!()
    format!(
        "
        mod {name} {{
        #[derive(Debug)]
        pub struct Actor {{
            running: bool,
            data: Data,
        }}
        #[derive(Debug)]
        pub struct Data {{
            {data}
        }}
        #[derive(Debug, PartialEq)]
        pub enum Input {{
            {gets}
        }}
        #[derive(Debug, PartialEq)]
        pub enum Output {{
            {sends}
        }}
        impl Actor {{
            pub fn start({optional_data_argument}) -> movie::Handle<
                std::thread::JoinHandle<()>,
                Input,
                Output
                >
            {{
                let (tx_ota, rx_ota) = std::sync::mpsc::channel(); // owner-to-actor
                let (tx_ato, rx_ato) = std::sync::mpsc::channel(); // actor-to-owner
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
                            use Output::*;
                            match message {{
                                {on_message}
                            }}
                        }};
                        while let Ok(message) = rx_ota.try_recv() {{
                            let reply: Output = on_message(message);
                            tx_ato.send(reply).unwrap();
                        }}
                        let reply: Option<Output> = {optional_tick_handler};
                        if let Some(reply) = reply {{
                            tx_ato.send(reply).unwrap();
                        }}
                        // sleep for 4 ms before polling or ticking
                        // 4 ms is minimum on some Linux systems
                        // so it was chosen for compatibility
                        use std::thread::{{spawn, sleep}};
                        use std::time::Duration;
                        sleep(Duration::from_millis(4));
                    }}
                }});
                movie::Handle {{
                    join_handle: handle,
                    tx: tx_ota,
                    rx: rx_ato,
                }}
            }}
        }}
        }}",
        name = attrs["name"],
        gets = attrs["gets"],
        sends = attrs["sends"],
        data = attrs["data"],
        on_init = attrs["on_init"],
        on_message = attrs["on_message"],
        optional_tick_handler = if tick {
            "Some(on_message(Input::Tick))"
        } else {
            "None"
        },
        optional_data_argument = if has_data { "data: Data" } else { "" },
        optional_default_data = if has_data { "" } else { "let data = Data {};" },
    )
    .parse()
    .unwrap()
}
