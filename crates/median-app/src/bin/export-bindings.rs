use specta_typescript::Typescript;

/// Regenerates the TypeScript IPC bindings the frontend imports as `@median/ipc`.
fn main() {
    median_app::bootstrap::specta_builder()
        .export(Typescript::default(), "packages/ipc/src/bindings.ts")
        .expect("failed to export IPC bindings");
    println!("wrote packages/ipc/src/bindings.ts");
}
