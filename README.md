# DragonDNS

DragonDNS is an open-souce self-hosted dynamic DNS service, configurable directly from the Cloudflare dashboard.

## Getting Started

Setting up DragonDNS with Docker Compose is incredibly simple, and only takes 6 lines:

```yaml
services:
  dragon-dns:
    image: ghcr.io/zinix-org/dragon-dns:latest
    environment:
      - TOKEN=<YOUR_CLOUDFLARE_TOKEN>
      - MACHINE_ID=1
```

You will also need a Cloudflare API token, with the `DNS: Edit` permission. Now you just need to run the container:

```sh
docker compose up -d
```

Now you just need to add the following string in the comments of the DNS records you want updated: `DDNS_ID=1`.

## Configuring

You can change the following environment variables to customize the behavior of DragonDNS:

| **Variable**  | **Usage**                                                                                  | **Default** |
| ------------- | ------------------------------------------------------------------------------------------ | ----------- |
| `TOKEN`       | Your Cloudflare API token. Needs the `DNS: Edit` permission.                               | None        |
| `MACHINE_ID`  | ID for updating records, identified by having `DDNS_ID=<MACHINE_ID>` in a records comment. | None        |
| `UPDATE_CRON` | Cron expression for when to look for IP address changes. Parsed by the [`croner`] crate.   | `* * * * *` |

All variables without a default value is required.

[`croner`]: https://lib.rs/crates/croner
