use crate::engine::Engine;

/// Android-facing wrapper that owns the core Engine.
///
/// Kept separate so JNI functions can be thin and avoid leaking core types into Java.
pub struct AndroidFrontend {
    engine: Engine,
}

impl AndroidFrontend {
    pub fn new() -> Self {
        Self { engine: Engine::new() }
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}
