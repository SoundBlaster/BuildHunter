# BuildHunter website deployment

The static landing, support and privacy pages are in `website/buildhunter/`.
The deployment workflow runs for changes on `main` or by manual dispatch and
publishes only to `https://egormerkushev.ru/buildhunter/`.

Create a GitHub Actions environment named `FTP` in this repository and set:

- Variables: `FTP_HOST` (the certificate-covered hostname), `FTP_PORT` (`21`),
  and `FTP_USER`.
- Secret: `FTP_PASSWORD`.

The target directory is fixed in the workflow to
`/www/egormerkushev.ru/buildhunter`; it must already exist and be accessible to
the configured FTP user. Deployment uses
FTPS with certificate verification, does not delete remote files, and verifies
published file hashes. It does not modify the Dreams website or its settings.
