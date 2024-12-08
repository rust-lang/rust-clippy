The tests present here are used to test the clippy lints page. The
goal is to prevent unsound/unexpected GUI (breaking) changes.

This is using the [browser-ui-test] framework to do so. It works as follows:

It wraps [puppeteer] to send commands to a web browser in order to navigate and
test what's being currently displayed in the web page.

You can find more information and its documentation in its [repository][browser-ui-test].

If you don't want to run in headless mode (helpful to debug sometimes), you can use
`DISABLE_HEADLESS_TEST=1`:

```bash
$ DISABLE_HEADLESS_TEST=1 cargo guitest
```

[browser-ui-test]: https://github.com/GuillaumeGomez/browser-UI-test/
[puppeteer]: https://pptr.dev/
