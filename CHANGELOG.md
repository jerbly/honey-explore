# 0.1.9

- Enums are now displayed in two ways:
  - If any brief is defined, the enum is displayed as a list of variants with their brief.
  - Otherwise, the compact form just showing the values is displayed.
- Examples for enums are now hidden since they're just repeats of the enum.
- Improvements to the display for clarity:
  - All branches are now shown before all leaves rather than mixed together.
  - Horizontal rules to separate attribute definitions.
  - Rework of Type with Examples on the same line.

# 0.1.8

- Pulling in improvements from [honeycomb-client](https://github.com/jerbly/honeycomb-client) 0.2.1
- Updated cargo-dist to 0.11.1

# 0.1.7

- Fix: Timerange in queries was sometimes rejected for being too long.

# 0.1.6

- Number type attributes now run an `avg` query when a dataset is clicked
- Template type attributes:
    - Now render with the `.<key>` suffix in the name
    - The dictionary, discovered from Honeycomb, is displayed in a `keys` section with query links
- Added a favicon


# 0.1.5

- Uses `cargo-dist` for build and release.

# 0.1.4

- Moved the Honeycomb API calls to a separate crate.
