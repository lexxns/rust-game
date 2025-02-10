# Rust Game
Building a game in Bevy just to play around and learn rust.

## Notes (Current Thoughts)
- GameState should be Immutable & we should keep a history of GameState
- Game Events should replace GameState with new GameState
- Re-joining clients should get an up-to-date GameState
- Implement an EventQueue, to trigger multiple Game Events in a row