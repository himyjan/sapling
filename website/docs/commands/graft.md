---
sidebar_position: 17
---

## graft
<!--
  @generated SignedSource<<2916ee1e69673f2f5c65cc2de31078a1>>
  Run `./scripts/generate-command-markdown.py` to regenerate.
-->


**copy commits from a different location**

Use Sapling&#x27;s merge logic to copy individual commits from other
locations without making merge commits. This is sometimes known as
&#x27;backporting&#x27; or &#x27;cherry-picking&#x27;. By default, graft will also
copy user and description from the source commits. If you want to
keep the date of the source commits, you can add below config to your
configuration file:

```
[tweakdefaults]
graftkeepdate = True
```

Source commits will be skipped if they are ancestors of the
current commit, have already been grafted, or are merges.

If `--log` is specified, commit messages will have a comment appended
of the form:

```
(grafted from COMMITHASH)
```

If `--force` is specified, commits will be grafted even if they
are already ancestors of, or have been grafted to, the destination.
This is useful when the commits have since been backed out.

If a graft results in conflicts, the graft process is interrupted
so that the current merge can be manually resolved. Once all
conflicts are resolved, the graft process can be continued with
the `-c/--continue` option.

`--from-path` and `--to-path` allow copying commits between
directories. Files in the grafted commit(s) outside of
`--from-path` are ignored. See `sl help directorybranching` for
more information.

The `-c/--continue` operation does not remember options from
the original invocation, except for `--force`.

Examples:

- copy a single change to the stable branch and edit its description:

```
sl goto stable
sl graft --edit ba7e89595
```

- graft a range of changesets with one exception, updating dates:

```
sl graft -D "0e13e529c::224010e02 and not 85c0535a4"
```

- continue a graft after resolving conflicts:

```
sl graft -c
```

- abort an interrupted graft:

```
sl graft --abort
```

- show the source of a grafted changeset:

```
sl log --debug -r .
```

See `sl help revisions` for more about specifying revisions.

Returns 0 on success.

## arguments
| shortname | fullname | default | description |
| - | - | - | - |
| `-r`| `--rev`| | revisions to graft|
| `-c`| `--continue`| `false`| resume interrupted graft|
| | `--abort`| `false`| abort an interrupted graft|
| `-e`| `--edit`| `false`| invoke editor on commit messages|
| | `--log`| | append graft info to log message|
| `-f`| `--force`| `false`| force graft|
| `-D`| `--currentdate`| `false`| record the current date as commit date|
| `-U`| `--currentuser`| `false`| record the current user as committer|
| `-d`| `--date`| | record the specified date as commit date|
| `-u`| `--user`| | record the specified user as committer|
| `-t`| `--tool`| | specify merge tool|
| `-n`| `--dry-run`| | do not perform actions, just print output|
