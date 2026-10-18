# Visual regression baselines

`scripts/visual-regression.py` renders the gallery (every design, light and dark, desktop and
phone sizes) and compares the screenshots with the baselines in a folder per platform:

| Folder | Rendered by |
| --- | --- |
| `macos/` | the `Visual regression` workflow on `macos-latest` (Metal) |
| `linux/`, `windows/` | `python3 scripts/visual-regression.py --update` on that platform |

Baselines depend on fonts, GPU and driver, so accept them from the same machine type that
compares them. To refresh the macOS baselines, run the workflow by hand with **update**
checked and replace `macos/` with the `visual-baselines` artifact.

Renders and diffs (changed pixels in red) are written to `target/visual/`.

The gallery renders into an offscreen image with its window hidden, so covered windows do not
matter, but the machine needs a GPU and an active graphics session (a logged in user). Single
color screenshots are reported as failures.
