extern crate proc_macro;
use proc_macro::TokenStream;

use std::collections::HashMap;

#[proc_macro]
// input: "SimplestActor gets : Ping , sends : Pong , on_message : Ping => Pong ,"
pub fn actor(input: TokenStream) -> TokenStream {
    let input = input.to_string();

    // locate attributes inside input string
    // (start, name, start_without_name)
    let mut locations = vec![(0, "name", 0)];
    let mut try_find = |attr| {
        let search_str = format!(" {} : ", attr);
        let pos = input.find(&search_str);
        if let Some(pos) = pos {
            locations.push((pos, attr, pos + search_str.len()));
        }
    };
    try_find("gets");
    try_find("sends");
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

    // assign default values for missing optional supported attrs
    attrs.entry("data").or_insert("".to_string());
    attrs.entry("on_init").or_insert("".to_string());

    let tick = false;

    format!(
        "
        struct {name} {{
            running: bool,
            {data}
        }}
        enum {name}Input {{
            {gets}
        }}
        enum {name}Output {{
            {sends}
        }}
        impl {name} {{
            fn on_init(&mut self) {{
                self.running = true;
                {on_init};
            }}
            fn start() -> movie::Handle<
                std::thread::JoinHandle,
                std::sync::mpsc::Sender<{name}Output>,
                std::sync::mpsc::Receiver<{name}Input>
            >
            {{
                let actor =
                let on_message = |message: {name}Input| {{
                    use {name}Output::*;
                    match message {{
                        {on_message}
                    }}
                }};
                use std::sync::mpsc::channel;
                use std::thread::{{spawn, sleep}};
                use std::time::Duration;
                let (tx_ota, rx_ota) = channel(); // owner-to-actor
                let (tx_ato, rx_ato) = channel(); // actor-to-owner
                let handle = spawn(move ||
                    while self.running {{
                        while let Ok(message) = rx_ota.try_recv() {{
                            let reply: {name}Output = on_message(message);
                            tx_ato.send(reply).unwrap();
                        }}
                        let reply: Option<{name}Output> = {optional_tick_handler};
                        if let Some(reply) = reply {{
                            tx_ato.send(reply).unwrap();
                        }}
                        sleep(Duration::from_millis(4));
                        // 4ms is minimum on some Linux systems
                        // so it was chosen for compatibility
                    }}
                );
                movie::Handle {{
                    join_handle: handle,
                    tx: tx_ota,
                    rx: rx_ato,
                }}
            }}
        }}",
        name = attrs["name"],
        data = attrs["data"],
        gets = attrs["gets"],
        sends = attrs["sends"],
        on_init = attrs["on_init"],
        on_message = attrs["on_message"],
        optional_tick_handler = if tick {
            "Some(on_message(Tick))"
        } else {
            "None"
        },
    )
    .parse()
    .unwrap()
}
