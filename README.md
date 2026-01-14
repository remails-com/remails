<a href="https://remails.com">
	<picture>
		<source media="(prefers-color-scheme: dark)" srcset="frontend/public/remails-logo-white.svg">
		<img src="frontend/public/remails-logo-black.svg" alt="Remails" />
	</picture>
</a>

# Remails

Remails is a transactional email platform built for developers.
With a clean API, modern developer tooling, and full transparency under the hood, Remails makes it easy to send reliable transactional emails.
Check it out at [remails.net](https://remails.net).



## Development

Run postgres using docker-compose:

```bash
docker compose up -d
cargo sqlx migrate run
````

Run `cargo test` to run the tests.
