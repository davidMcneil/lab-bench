# Lab Bench

Run GitLab queries and view the results in a list

## Dev

Run for the web. First you have to comment out `base_path = "slotted-pig"` in Dioxus.toml
> dx serve --hot-reload

If you are editing CSS you need to run the below to have `assets/tailwind.css` automatically updated.
> npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch

Create a build for the web
> dx build --release
