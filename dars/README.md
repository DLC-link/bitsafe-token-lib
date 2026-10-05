# DAR packages

A participant node must hold these DAR packages before its parties can mint
or burn (redeem) an asset.

| Folder | Holds |
|---|---|
| `dependencies/` | The Splice and Canton Network Utility DARs that every asset uses |
| `cbtc/` | The `cbtc` DARs from 1.0.0 to 1.2.1 |
| `beth/` | The `beth` 0.2.0 DAR |

The asset DARs are the released `cbtc` and `beth` packages. The dependency
DARs are the Splice and Canton Network Utility packages that they use.

Each asset DAR has this main package id and this SHA-256 checksum:

| DAR | Package id | SHA-256 |
|---|---|---|
| `cbtc-1.0.0.dar` | `43a8452a56388d22c6058abe03e90dadbae9a20a682634568b07a93531dda1a3` | `1b33bc324def3a66559e5e8a83833de59dbfaed56c04c52d45130dbd6a40920d` |
| `cbtc-1.1.0.dar` | `61ed690af72fda469c2a2df960d81bf59be5ff8d0f4844e816944b5fce267d92` | `d72828995cd1442858badc1779c9d17698dd60fbf2b9820a6aaedff16aed60bc` |
| `cbtc-1.1.1.dar` | `e0b1ae1cf5bb4bcff668df849c397d551065cfc0563a69c6b106a639f4f43b2c` | `c3d04576369da9304cb4dfc236676d292fe4c3ea5e7092cf10e6c1a31680dbaa` |
| `cbtc-1.2.0.dar` | `f240dd5d1a98079f37c0f93272cf5b28d4523027c42d0003c4c7a530eed6c313` | `4519ada05678f13cd10c3953b07e625a3f36e4fac186f8cf80e6b095a3c3a82d` |
| `cbtc-1.2.1.dar` | `3484f3214004982cc14315f7835d8722af9b141aebfc3ddcc605b2bee338f1bd` | `b6f82dcf721e94fb1b0d930dc6707ee15a7a6f82e88b4cbb1abc9a40ce2a7a76` |
| `beth-0.2.0.dar` | `99a39f943ce8fcb39159f8ed4fabab8c71da14db932347cddcf88bf0ac6e521f` | `f61e84e38a51e54855b20210d20acb32b1b566fb30a5a92ac4d163323d079c9c` |

The utility DARs come from Digital Asset. The file
`dependencies/Terms.and.Conditions.for.Canton.Network.Utility.txt` sets the
terms of their use.

## Check a participant

The `check_dars` example compares the newest package of each DAR in
`dependencies/` and in the asset's folder with the packages on the
participant. `ASSET` names the asset, and
[`examples/README.md`](../examples/README.md) lists the other variables:

```bash
ASSET=cbtc cargo run --example check_dars
ASSET=beth cargo run --example check_dars
```

A library caller does the same with `bitsafe_token::check_dars`. Its `root`
argument is the folder that holds `dars/`. Inside this repository, that is
`env!("CARGO_MANIFEST_DIR")`. Outside it, that is a clone of this repository
at the same tag.

## Upload with gRPC

`upload_dars.sh` uploads the DARs that one asset needs: first
`dependencies/`, then the asset's folder.

You need:

- `grpcurl` and `jq`
- a port forward to the participant admin API, by default `localhost:5002`
- a participant admin token, unless the admin API checks no token

```bash
export jwt_token="<participant admin token>"           # optional
export canton_admin_api_url="localhost:5002"           # optional
dars/upload_dars.sh cbtc
dars/upload_dars.sh beth
```

The script stops at the first DAR that fails to upload. Canton skips a DAR
that the participant already holds, so a second run is safe. `grpcurl` reads
the token from the environment, so the token does not appear in the process
list.

## Upload with the Canton console

The Scala scripts upload and list the CBTC DARs only. They need the setup in
[scala-prerequisites.md](scala-prerequisites.md). Run them from this folder:

```bash
cd dars
canton run 00_UploadDars.sc -c ./misc/connect.conf
canton run 00_ValidateDars.sc -c ./misc/connect.conf
```

`misc/connect.conf` holds an empty `token`. Do not write a token into it,
because git tracks the file. Copy it to a folder outside this repository,
add the token to the copy, and pass the copy with `-c`.
