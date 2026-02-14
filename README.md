<div align="center">
    <img src="https://img.shields.io/github/license/temper-mc/temper?style=for-the-badge&color=4A0D67" alt="License">
    <img src="https://img.shields.io/github/languages/code-size/temper-mc/temper?style=for-the-badge&color=8332AC" alt="Code Size">
    <img src="https://www.aschey.tech/tokei/github.com/temper-mc/temper?style=for-the-badge&color=caa8f5" alt="Lines of Code">
    <img src="https://img.shields.io/github/last-commit/temper-mc/temper?style=for-the-badge&color=07BEB8" alt="Last Commit">
    <a  href="https://discord.gg/6QPZgUy4sA">
    <img alt="Discord" src="https://img.shields.io/discord/1469329170055037087?style=for-the-badge&logo=discord&logoColor=73EEDC&color=73EEDC&link=https%3A%2F%2Fdiscord.gg%2F6QPZgUy4sA">
    </a>
</div>

## About

Temper is a Minecraft server implementation written in Rust, with the goals of being extremely fast and memory
efficient, while also being easy to use and set up. With a focus on community, extensibility and performance, we hope to
create a server that can be used by everyone from casual players to large server owners.

Originally a fork of the FerrumC project, Temper aims to supersede its predecessor by providing a more polished and
user-friendly experience, while still maintaining the same high standards for performance and efficiency. We are
committed to building a strong community around Temper and welcome contributions from developers of all skill levels.

<img src="https://github.com/temper-mc/temper/blob/master/assets/README/in_game.png?raw=true" alt="In-game screenshot">

## Project Links

* **Official Website:** **[temper-mc.com](https://www.temper-mc.com)**
* **Discord Community:** **[Join our Discord](https://discord.gg/6QPZgUy4sA)**
* **GitHub Repository:** **[temper-mc/temper](https://github.com/temper-mc/temper)**

## Key Features and goals

- **High Performance**: Temper is built with performance in mind, utilizing Rust's powerful features and optimizations
  to offer the best possible performance.
- **Memory Efficiency**: Temper is designed to use as little memory as possible, making it suitable for servers of all
  sizes.
- **Ease of Use**: One major goal of Temper is to be easy to set up and use, even for those who may not be familiar with
  server administration. We aim to make setup and administration as straightforward as possible, while still providing
  the flexibility and power that advanced users need.
- **Community Focus**: We want to build a strong community around Temper, and we welcome contributions from developers
  of all skill levels. We believe that a strong community is essential for the success of a project like this, and we
  are committed to fostering a welcoming and inclusive environment for all contributors.
- **Quality and Stability**: We are committed to providing a high-quality and stable server implementation. We will
  prioritize fixing bugs and improving stability over adding new features, and we will always strive to maintain a high
  standard of quality in our codebase.
- **Maintainable Codebase**: We want to maintain a clean and well-organized codebase that is easy to understand and
  contribute to. We will follow best practices for code organization and documentation, and we will strive to make our
  code as readable and maintainable as possible. We will also prioritize code reviews and testing to ensure that our
  codebase remains healthy and maintainable over time.

## Getting Started

### Installing a pre-compiled binary

While it is recommended to compile from source for the best performance and latest features, we understand that not
everyone may be comfortable with that process. Therefore, we provide pre-compiled binaries for Windows, Linux, and macOS
on our Github Actions.
At a later date we hope to make this easier by providing downloads on the Releases page, but for now you can follow the
instructions below to download the latest pre-compiled binary.

[//]: # (#### Option 1: Download pre-compiled binary &#40;Maybe outdated!&#41;)

[//]: # ()

[//]: # (1. Go to the [Releases]&#40;https://github.com/temper-mc/temper/releases&#41; page)

[//]: # (2. Download the latest version for your operating system)

[//]: # (3. Extract the archive to your desired location)

1. Go to the [Actions](https://github.com/temper-mc/temper/actions) tab
2. Click on the latest build
3. Scroll all the way down to the `Artifacts` section
4. Download the artifact for your operating system (Windows, Linux, or macOS)
5. Follow the instructions in the `Usage` section

### Compile from source

##### Clone and build the project.

```bash
# Clone the repository
git clone https://github.com/temper-mc/temper
cd temper

# Build the project
cargo build --release
```

#### The binary will be in target/release/

## Usage

```plaintext
Usage: temper.exe [OPTIONS] [COMMAND]

Commands:
setup   Sets up the config
import  Import the world data
run     Start the server (default, if no command is given)
help    Print this message or the help of the given subcommand(s)

Options:
--log <LOG>  [default: debug] [possible values: trace, debug, info, warn, error]
-h, --help       Print help
--no-tui         Disable the TUI (terminal user interface)
```

1. Move the Temper binary (`temper.exe` or `temper` depending on the OS) to your desired server directory
2. Open a terminal in that directory
3. (Optional) Generate a config file: `./temper setup`
    - Edit the generated `config.toml` file to customize your server settings
4. Run the server:
    - Windows: `.\temper.exe`
    - Linux/macOS: `./temper`

## Development

We welcome contributions! If you'd like to contribute to Temper, please follow these steps:

1. Fork the repository
2. Create a new branch for your feature
3. Implement your changes
4. Write or update tests as necessary
5. Submit a pull request

The Discord server is where most of the development discussion happens, so feel free to join and ask any questions you
may have or discuss your ideas with the community.

## FAQ

### How does this project differ from:

- **Paper/Spigot/Bukkit**: These projects are the cornerstone of the Minecraft server ecosystem, and they have been
  around for a long time. However, they are all written in Java and are based on the vanilla server codebase, which is
  not very performant and has a lot of technical debt. We are taking a different approach by writing the server from
  scratch in Rust, which allows us to take advantage of Rust's performance and safety features to create a much more
  efficient and maintainable server implementation.
- **Pumpkin**: Pumpkin's goal is to simply reimplement the vanilla server in Rust, but they are taking the approach of
  just porting decompiled Java code to Rust. While easier to implement, this approach is not very maintainable and will
  likely lead to a lot of technical debt and performance issues. We are taking a more holistic approach to the server
  implementation, which lets the strengths of Rust shine through and allows us to make design decisions that are best
  for the project rather than just porting a 17-year-old codebase.
- **FerrumC**: FerrumC is our predecessor and the project that Temper was originally forked from. While we share the
  same goals of performance and efficiency, Temper aims to shoot above and beyond what FerrumC was able to achieve by
  focusing on open source values, community involvement, and a more polished and user-friendly experience. We are
  committed to building a strong community around Temper and welcome contributions from developers of all skill levels,
  while also maintaining a high standard of quality and stability in our codebase.

### Will we be implementing terrain generation?

Yes! We do have a somewhat simplistic terrain generator implemented right now for demonstration purposes, but we do plan
to implement a more robust and feature-rich terrain generator in the future. Whether that will end up being 1:1 with the
vanilla generator or something custom is still up in the air, but we are open to suggestions and ideas from the
community on how to best implement this.

### Will there be plugins? And how?

We do very much plan to have a plugin system and as of right now we are planning to use
some kind of FFI (foreign function interface) to allow for plugins to be written in Rust. Plugins are not our top
priority
right now, and we want to make sure the API is designed well before we start implementing it to avoid breaking changes
later.
We are open to suggestions and ideas from the community on how to best implement this.

### Will I be able to use plugins or mods from paper/spigot/bukkit/forge/fabric etc.?

No. Even if we did implement a perfect 1:1 API match for the vanilla server, the underlying implementation is still
completely different.
Java plugins and mods rely heavily on Java features such as reflection and dynamic class loading, which simply aren't
possible in Rust.
If we made a Java translation layer, it would be extremely slow and only the most basic plugins and mods would work. If
a plugin
or mod is basic enough to work through a translation layer, it would be much better to just rewrite it in Rust for
performance
and compatibility reasons.

### Why did we fork from FerrumC?

We decided to carry on the grand tradition of forking a Minecraft server implementation over management disagreements.
In all seriousness, we had some differences in vision and approach to the project, and we felt that it would be best to
start fresh with a new repository and a new name. We are grateful for the work that was done on FerrumC, and we hope to
build on that foundation while also taking the project in a new direction that we are excited about.

## License

FerrumC was licensed under the MIT License, but Temper has moved to the GNU General Public License v3.0 (GPL-3.0) to
better align with our values of open source and community involvement. The GPL-3.0 is a copyleft license that requires
any derivative works to also be licensed under the same terms, which we believe will help to ensure that Temper remains
free and open for everyone to use and contribute to.

Due to this, commits to FerrumC and prior to 14/02/2026 are licensed under the MIT License, while commits to Temper and
after 14/02/2026 are licensed under the GPL-3.0. This is not a dual license, rather a change in license that occurred at
a specific point in time.

## Star History

<a href="https://star-history.com/#temper-mc/temper&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=temper-mc/temper&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=temper-mc/temper&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=temper-mc/temper&type=Date" />
 </picture>
</a>

## Stats

![Alt](https://repobeats.axiom.co/api/embed/f28c7e31c2d3b037ca37493bea8a65cbf1021275.svg "Repobeats analytics image")
