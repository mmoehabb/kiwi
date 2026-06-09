// I am exploring if `ui.button("Record Sample").clicked()` triggers multiple times per click in egui.
// Egui buttons typically trigger once per click.
// However, maybe the channel receives multiple events if the event loop executes fast?
// No, `clicked()` is true exactly once per click down-up cycle.
