use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;
use kanal::AsyncReceiver;

pub fn app(cx: Scope) -> Element {
    let mut count = use_state(cx, || 0);

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
}

pub fn launch(receiver: AsyncReceiver<bool>) {
    LaunchBuilder::new(app).launch()
}
