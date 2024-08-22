# FT-DDNS

Florence Telecom's Dynamic DNS middleware for Route53

This program is meant to serve as a secure middleware to more easily distribute access to Route53's interface for updating DNS records.

It supports password based authentication on secure channel (HTTPS) as well as signature based authentication for unsecure channel (HTTP) for older device which do not support HTTPS.

This program only supports IPv4 addresses as of right now.

## Set up

### Database

The program requires a database to connect to (Postgres, MySQL/MariaDB and SQLite supported)

### Environment variables

To setup the program, you need to set the following environment variables:

- `HOSTED_ZONE_ID_LIST`: Route 53's hosted zone IDs for the hosted zones you want to allow the service to use. Values are separated by `;`
- `DATABASE_URI`: A valid database URI to connect to your database server.
- `FT_DDNS_BASE_URL`: The public URL of the service, used to automate script generation
- `DDNS_ADMIN_PASSWORD`: (Optional) The password to bootstrap in the database for creating the `admin` account, highly recommended on first startup
- `LOG_LEVEL`: (Optional) The log level desired for the program (`DEBUG`, `INFO`, `WARN`, `ERROR`, `OFF`)
- `SKIP_MIGRATION`: (Optional) Set variable to anything in order to skip database migrations. Could be useful after updates if you don't want to update the database, or to speed up the initialization. 

### Reverse proxy

It is also recommended to use a reverse proxy with the following bindings:

- HTTP traffic to `/unsecure`
- HTTPS traffic to `/secure`
- HTTPS traffic with `/mgmt` to `/mgmt`

Important: The reverse proxy **MUST** set the HTTP header `X-Real-Ip` to the IP of the client for the program to be able to update the IP correctly. If this is not done, the domain will be set to use the internal IP of the reverse proxy.

I plan on doing a `compose.yml` file at some point which will include a set of the required configuration to get up and running including a database container, a reverse proxy and the program itself.

### AWS 

You must create an IAM policy in AWS to allow the program's role to modify the hosted zone for which you want to use the dynamic DNS service and give the program's environment access to said policy with a role. The policy needs to have the permissions to use  [`ListHostedZones`](https://docs.aws.amazon.com/Route53/latest/APIReference/API_ListHostedZones.html) as well as [`ChangeResourceRecordSets`](https://docs.aws.amazon.com/Route53/latest/APIReference/API_ChangeResourceRecordSets.html) for each of the hosted zone defined in the environment variables .

## Usage

### Management

To use the management routes, you must authenticate using basic authentication.

#### Routes

`GET /mgmt/add-domain/password/<domain>`: Creates a new account, and returns the newly generated password.

`POST /mgmt/add-domain/signing/<domain>`: Creates a new signing account, must add the public key in the body of the request.

`POST /mgmt/admin/new`: Allows the `admin` account to create new users which can create accounts using the two aforementioned routes. Requires a JSON body with the fields `username` and `password` set to make the account.

### Password based authentication

To use password based accounts, the request must be authenticated using basic authentication with the username being the domain created previously, and the password being what was the output at the account creation.

If you intend on using this method of authentication, it is important to have a SSL certificate with the reverse proxy you are using.

#### Routes

`GET [/secure]/nic/update`: Updates the domain to use the IP that was requested.

### Signing based authentication

To use signing based authentication, you must create a RSA keypair. The following commands can create the public and private file:

```shell
openssl -genrsa -out private.pem 4096

openssl -rsa -in private.pem -pubout -out public.pem
```

The public file created must be used to create the account by having the content of the file in the request body.

#### Routes 

`GET [/unsecure]/nic/update`: Updates the domain to use the IP that was requested.

Requires the following HTTP headers:

- `Ftddns-Date`: [RFC-3339](https://datatracker.ietf.org/doc/html/rfc3339#section-5.8) formatted date. Can be generated using 

  ````shell
  date --rfc-3339=seconds
  ````

- `Ftddns-Domain`: The domain for which the account is created.

- `Ftddns-Signature`: A Base64 encoded signature yielded from the private key signing the date and the domain joined by a semi-column using SHA-256 digest.
  The signature can be generated with the following command:

  ```shell
  echo -n "$DATE;$DOMAIN" | openssl dgst -sha256 -sign $PRIVATE_KEY | openssl base64 | tr -d "\\n"
  ```

## Building

By default, the program will build with drivers for every supported database, but you can disable default features and select only the database types you desire. If you plan on building for another platform, you can set OpenSSL to be built into the binary instead of linked. For this enable the `openssl-vendored` feature flag. For development work, you can enable the `read_only_aws` feature to stop the program from sending update requests to AWS.

My principle use case is to use it in a lightweight Alpine Linux container. For this reason, I build the container with only the database driver I need, as well as `openssl-vendored` to facilitate cross-compilation.



## Planned features

- [ ] Automatic configuration of own domain name on startup
- [ ] Caching of IP to not do unnecessary writes and better tracking of changes
- [ ] Endpoint served deployment scripts for easy installation
