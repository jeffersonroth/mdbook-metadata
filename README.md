<!-- PROJECT LOGO -->
<br />
<p align="center">
  <a href="https://github.com/jeffersonroth">
    <img src="https://raw.githubusercontent.com/jeffersonroth/common-assets/main/assets/images/logo.svg" alt="Logo" width="80" height="80">
  </a>

  <h3 align="center">mdBook Metadata Preprocessor</h3>

  <p align="center">
    mdBook preprocessor to parse markdown metadata.
  </p>
</p>

<!-- TABLE OF CONTENTS -->
<details open="open">
  <summary>Table of Contents</summary>
  <ol>
    <li><a href="#getting-started">Getting Started</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
  </ol>
</details>

<!-- GETTING STARTED -->

## Getting Started

1. Install the cli tool:

   ```sh
   cargo install mdbook-metadata
   ```

2. Add the preprocessor to your `book.toml` file:

   ```toml
   [preprocessor.metadata]
   valid-tags = ["title", "author", "keywords", "released"]
   default-author = "Jane Doe"
   continue-on-error = true # default: true
   ```

3. Add metadata to your markdown file:

   ```markdown
   ---
    title: Example
    author: Your Name
    keywords: tag1, tag2
    released: false
    ---

    # Example
   ```

4. Build your book and serve it locally:

   ```sh
   mdbook serve --hostname 0.0.0.0
   ```

5. Verify the rendered html head tags are correct (title and meta).

<!-- LICENSE -->

## License

Copyright (C) 2024 Jefferson Johannes Roth Filho. See `LICENSE` for more information.

<!-- CONTACT -->

## Contact

Jefferson Roth - <jjrothfilho@gmail.com>

Project Link: [https://hub.docker.com/r/jeffroth/mdbook-metadata](https://hub.docker.com/r/jeffroth/mdbook-metadata)

crates.io: [mdbook-metadata](https://crates.io/crates/mdbook-metadata)
