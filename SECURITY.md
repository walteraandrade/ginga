# Security policy

## Supported versions

The latest published version of each crate gets fixes, and so does the `main` branch.

## Reporting a vulnerability

Please don't open a public issue. Report it privately through GitHub instead: on the repository's **Security** tab, choose **Report a vulnerability** ([direct link](https://github.com/walteraandrade/ginga/security/advisories/new)). Only the maintainers see the report.

Say what is affected (`ginga`, `ginga-gpui`, or both, and the commit or version), what an attacker can do, and how to reproduce it; a short test or a `Trace` helps.

You should hear back within a week. Once a fix is ready, it is released with an advisory that credits you, unless you'd rather not be named.

## Scope

The engine, `ginga`, has no dependencies and no `unsafe` code. The adapter, `ginga-gpui`, pins one version of GPUI (`gpui-pre`); a problem in GPUI itself belongs to [GPUI's maintainers](https://github.com/zed-industries/zed), though we would like to hear about it too if it reaches ginga's users.
