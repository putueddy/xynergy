# Frontend Build Setup

## Tailwind CSS Configuration

Tailwind CSS v4 is configured and ready to use with Leptos.

### Build Commands

```bash
# Navigate to frontend directory
cd src/frontend

# Build CSS for production (minified)
npm run build

# Watch for changes during development
npm run watch

# Check what was built
ls -lh public/output.css
```

### File Structure

```
src/frontend/
├── package.json          # npm configuration
├── postcss.config.js     # PostCSS configuration
├── style/
│   └── tailwind.css     # Input CSS with @import and @theme
└── public/
    ├── index.html       # HTML template
    └── output.css       # Generated CSS (don't edit manually)
```

### Tailwind CSS v4 Changes

**Key differences from v3:**

1. **No `tailwind.config.js`** - Configuration is done in CSS using `@theme`
2. **New import syntax** - Use `@import "tailwindcss"` instead of `@tailwind` directives
3. **New custom utilities** - Use `@utility` instead of `@layer components`
4. **CSS variables** - All theme values are CSS variables (e.g., `--color-blue-500`)
5. **New CLI** - Uses `@tailwindcss/cli` package

### CSS Structure

```css
@import "tailwindcss";

@theme {
  /* Define custom theme variables */
  --color-primary-500: #3b82f6;
  /* ... */
}

@utility btn {
  /* Custom utility class */
  padding: 0.5rem 1rem;
  /* ... */
}
```

### Usage in Leptos

```rust
view! {
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
        <h1 class="text-2xl font-bold text-blue-600">
            "Hello Tailwind v4!"
        </h1>
        <button class="btn-primary">
            "Click me"
        </button>
    </div>
}
```

### Custom Components

The following custom utility classes are defined in `style/tailwind.css`:

- `.btn` - Base button styles
- `.btn-primary` - Blue primary button
- `.btn-secondary` - Gray secondary button  
- `.card` - Card container with shadow
- `.input` - Form input styles

### Development Workflow

1. **Start CSS watcher** (Terminal 1):
   ```bash
   cd src/frontend
   npm run watch
   ```

2. **Build and run Leptos** (Terminal 2):
   ```bash
   # For CSR mode
   cargo build --package xynergy-frontend
   
   # For SSR mode (requires cargo-leptos)
   cargo leptos serve
   ```

### Production Build

```bash
cd src/frontend
npm run build  # Creates minified output.css
cd ../..
cargo build --release --package xynergy-frontend
```

### Upgrading from v3

If you're familiar with Tailwind v3, here are the key changes:

1. **Remove `@tailwind` directives** - Replace with `@import "tailwindcss"`
2. **Move config to CSS** - Use `@theme` block instead of `tailwind.config.js`
3. **Update custom utilities** - Use `@utility` instead of `@layer components`
4. **Use CSS variables** - Reference theme values with `var(--color-blue-500)`
5. **Install new CLI** - Use `@tailwindcss/cli` package

### Troubleshooting

**CSS not updating?**
- Make sure `npm run watch` is running
- Check that classes are spelled correctly
- Verify the CSS file is being scanned

**Dark mode not working?**
- v4 uses `prefers-color-scheme` by default
- Or add `dark` class to HTML element

**Build errors?**
- Ensure you're using `@tailwindcss/cli` v4
- Check Node.js version: `node --version` (needs 18+)
- Verify all packages are v4: `npm list | grep tailwind`

### Resources

- [Tailwind CSS v4 Documentation](https://tailwindcss.com/docs)
- [Upgrade Guide](https://tailwindcss.com/docs/upgrade-guide)
- [v4 Changes](https://tailwindcss.com/docs/upgrade-guide#changes-from-v3)
