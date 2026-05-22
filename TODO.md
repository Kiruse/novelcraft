- [ ] Introduce new "aside" chat for temporary brainstorming sessions
- [ ] Introduce multiple timelines
- [ ] Introduce events to pages & use them to materialize game state
- [ ] v0.2: Rewrite the entire game logic in Rust

Rewriting Game Logic in Rust has various benefits:

1) Currently, we've reinvented CRUD for resources like sessions, state snapshots, pages, etc.
   Rewriting it in Rust such that the frontend simply passes thru user input means we can vastly
   reduce the commands footprint to only handle user input rather than resource management.
2) With the game logic in Rust it becomes dramatically harder for hostile projects to simply
   fork our project and deploy as their own service (paid or freemium) online.
3) It makes for a much better portfolio project for me when applying for Rust roles.
4) We will eventually need the game engine in Rust anyways so we can use it as the RL environment
   for our custom-trained LLM. Also makes unit-testing feasible.
