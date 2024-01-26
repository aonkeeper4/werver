# werver

## wow look its a web server. isnt that cool

um

so. procedural macros anyone

hi! this is a (extremely extremely minimal) web server i made! (however this is still probably my biggest project in rust to date)
i started with the template given by [the rust book chapter 20](https://doc.rust-lang.org/stable/book/ch20-00-final-project-a-web-server.html), added minor error handling and a `#[route]` attribute proc macro and here we are :catsnug:

## todos

- ~~rework `lazy_static` integration~~ boom done i just expanded the macro namually :peaceline:
- ~~make sure everything is tidy (all generated imports qualified correctly, file stuff)~~ should be question mark
  - ~~move `main.rs` to outer crate and make this a lib~~ done ! we have examples via `cargo run --example <example_name>` now :3
- ~~fix where the import errors come from~~ done! unless im missing other cases of this
- generally clean stuff up (code quality review)
  - check whether i actually need all those `clone`s (im suspicious i do)
  - read up on how to actually do multithreaded stuff in rust and rework as needed (surely it wont be too much :clueless:) (god this got . so much worse when i tried to implement proper error handling :catplant:)
  - tidy up apis
    - generally just "should this be public" and "are my function signatures cool and based or cringe as hell"
- rework codegen in `#[route]` (im sure theres better ways to do everything i did)
  - just dicovered `quote_spanned` exists im sure its useful
- ~~do something about error handling in `HttpServer::listen`~~ ok this is mostly implemented there are a few bugs to work out that i think are gonna be annoying to find tho ,,
- add support for more actual web features
  - different request types
  - more fully-featured responses
  - serve other stuff than just bare html
- route trees? subroutes? routes with variable arguments??
  - more attribute macros oooohhh
- make errors in macros more descriptive
  - figure out and use proper spanned errors
- add all the cool stuff to my cargo.toml
  - like descriptions and docs and repo link n stuff
    - ohhhh fuc k. i have to write docs for everything :catresort:
- publish to crates.io!! (once ive got at least most of this)
