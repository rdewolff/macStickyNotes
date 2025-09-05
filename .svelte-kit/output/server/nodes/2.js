

export const index = 2;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/_page.svelte.js')).default;
export const imports = ["_app/immutable/nodes/2.CKa0g3uD.js","_app/immutable/chunks/Bzak7iHL.js","_app/immutable/chunks/25g6-NV2.js","_app/immutable/chunks/CRj6ZoC0.js"];
export const stylesheets = ["_app/immutable/assets/2.D37z8JIA.css"];
export const fonts = [];
