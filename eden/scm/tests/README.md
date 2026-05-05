# Sapling .t Tests

Shell-script tests with inline expected output. Most use `debugruntest`, a Python-based shell emulator (`sapling/testing/`) that runs .t tests in-process without depending on `/bin/bash`. This gives fast execution and cross-platform portability.

**WATCH OUT:**
- ALWAYS use `newclientrepo` (never raw `hg init`)
- Prefer `drawdag` over manually creating commits -- it's concise and deterministic
- Create a new repo for each sub-test-case
- Avoid `..` in paths; use `$TESTTMP/repo1`

## .t File Format

| Prefix | Meaning |
|--------|---------|
| `  $ ` | Shell command |
| `  > ` | Command continuation (heredoc, multi-line) |
| `  ` (2 spaces, no $) | Expected output |
| No indent | Comment/description |
| `#require` | Skip test if feature missing |
| `#if`/`#else`/`#endif` | Conditional blocks |
| `#testcases` | Define test variants |
| `  >>> ` | Python inline code (debugruntest only) |

Python `>>>` blocks execute inline Python. Non-None expressions print automatically. Side effects persist across blocks within a test.

## Test Runner Directives

| Directive | Effect |
|-----------|--------|
| `#debugruntest-incompatible` | Falls back to legacy `run-tests.py` with real bash. For tests incompatible with debugruntest's shell emulation -- should never be used for new tests. |
| `#inprocess-hg-incompatible` | Spawns a new Sapling process per `hg`/`sl` invocation instead of running in-process. Needed when Python global side effects (e.g. `uisetup()`) persist across invocations. Much slower -- avoid if possible. |

## Output Modifiers

| Modifier | Purpose |
|----------|---------|
| `(glob)` | Glob pattern matching |
| `(re)` | Regex matching |
| `(esc)` | Escape sequences |
| `(?)` | Optional line |
| `(feature !)` | Required only if feature present |
| `[N]` | Expected non-zero exit code |

## Key Helper Functions (from `tinit.sh`)

| Function | Purpose |
|----------|---------|
| `newclientrepo` | Create and cd into a new repo (ALWAYS use this) |
| `eagerepo` | Set modern client config |
| `drawdag` | Create commit graphs from ASCII art |
| `setconfig` | Set hg config |
| `enable`/`disable` | Enable/disable extensions |
| `configure modernclient` | Use modern client defaults |
| `showgraph`/`tglog` | Display commit graph |

## Writing .t Tests

```
Test description here

  $ newclientrepo

Create some files:
  $ echo "hello" > file1.txt
  $ hg add file1.txt
  $ hg commit -m "initial"

Verify status is clean:
  $ hg status

Test modification:
  $ echo "world" >> file1.txt
  $ hg status
  M file1.txt

Test with commit graph:
  $ newclientrepo <<'EOS'
  > B
  > |
  > A
  > EOS

  $ hg log -r "all()" -T "{desc}\n"
  A
  B
```

## Test Stubs

### Basic test with drawdag

```
  $ newclientrepo
  $ drawdag <<'EOS'
  > C
  > |
  > B
  > |
  > A
  > EOS

  $ hg log -r "all()" -T "{desc}\n"
  A
  B
  C
```

### Remote clone (lazy repo)

```
  $ newremoterepo
  $ drawdag <<'EOS'
  > B
  > |
  > A
  > EOS

  $ cd $TESTTMP
  $ clone server client1
  $ cd client1
```

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `$TESTTMP` | Temporary directory (auto-substituted in output) |
| `$HGRCPATH` | Test's hgrc config file |
| `$HGPORT`, `$HGPORT1`, `$HGPORT2` | Available TCP ports |

## `#require` Feature Gates

| Feature | Effect |
|---------|--------|
| `eden` / `no-eden` | Only run with/without EdenFS |
| `fsmonitor` / `no-fsmonitor` | Only run with/without Watchman |
| `symlink` / `execbit` | Only run on platforms supporting these |
| `windows` / `no-windows` | Windows only / exclude Windows |
| `linux` / `osx` / `linuxormacos` | Platform restriction |
| `git` | Only run when Git is available |
| `slow` | Marks test as slow |

## .t Test Variants (Buck Targets)

| Target | Environment | When to Use |
|--------|-------------|-------------|
| `sl_run_tests` | Plain Sapling (no EdenFS, no Watchman) | Default: pure Sapling logic changes |
| `sl_edenfs_run_tests` | With EdenFS (`HGTEST_USE_EDEN=1`) | Changes affecting EdenFS integration |
| `sl_watchman_run_tests` | With Watchman filesystem monitoring | Changes affecting fsmonitor/status |
| `sl_mononoke_run_tests` | With Mononoke server backend | Changes affecting remote repo interaction |

## Running .t Tests

```bash
# Local iteration (debugruntest tests):
buck run @fbcode//mode/opt fbcode//eden/scm:hg -- .t tests/test-foo.t

# Via Buck:
buck test fbcode//eden/scm/tests:sl_run_tests -- test_foo_t
```

## Updating Expected Output (Autofix)

When code changes cause .t test output to change:
1. Run the test — it creates a `.t.err` file with actual output
2. Auto-update with `-i` flag:
   ```bash
   buck run @fbcode//mode/opt fbcode//eden/scm:hg -- .t tests/test-foo.t -i
   ```
3. Review the diff: `sl diff tests/test-foo.t` — verify output changes are correct, not masking bugs
4. For `#debugruntest-incompatible` tests: `./run-tests.py -i test-foo.t`

## Legacy .t Tests

Tests marked with `#debugruntest-incompatible` use real bash and may behave inconsistently across operating systems and shell utility versions. Ideally all tests would be migrated to debugruntest, but some incompatibilities remain.

Some tests also specify `#modern-config-incompatible`, which opts out of the modern feature set. Ideally these tests migrate to modern configuration, but migration is tedious.

## Doc Tests

Some Python functions include tests embedded in their doc string:

```python
def dedup(items):
    """Remove duplicated items while preserving item order.

    >>> dedup([1,2,3,2])
    [1, 2, 3]
    >>> dedup([3,2,1,2])
    [3, 2, 1]

    """
    return list(collections.OrderedDict.fromkeys(items))
```

Run using: `buck run @fbcode//mode/opt fbcode//eden/scm:hg -- .t doctest:sapling.util`

## Naming Conventions

- `.t` tests: `test-<feature>.t` (hyphenated, must start with `test-`)
- Buck target filter: replace `-` and `.` with `_` (e.g., `test-rust-checkout.t` -> `test_rust_checkout_t`)

## Internals

For details on debugruntest's shell emulation, see `sapling/testing/__init__.py`.
