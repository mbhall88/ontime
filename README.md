# ONTime

[![Rust CI](https://github.com/mbhall88/ontime/actions/workflows/ci.yaml/badge.svg)](https://github.com/mbhall88/ontime/actions/workflows/ci.yaml)
[![Crates.io](https://img.shields.io/crates/v/ontime.svg)](https://crates.io/crates/ontime)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![github release version](https://img.shields.io/github/v/release/mbhall88/ontime)](https://github.com/mbhall88/ontime/releases)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.7533053.svg)](https://doi.org/10.5281/zenodo.7533053)

Extract subsets of ONT (Nanopore) reads based on time

- [Motivation](#motivation)
- [Install](#install)
- [Examples](#examples)
- [Usage](#usage)
    - [Time range format](#specifying-a-time-range)
  - [Usage with Dorado output](#usage-with-dorado-output) 
- [Cite](#cite)

## Motivation

Some collaborators wanted to know how long they need to perform sequencing on the
Nanopore
device until they got "sufficient" data (sufficient is obviously application-dependent).

They were just going to do multiple runs for different amounts of time. So instead, I
created `ontime`
to easily grab reads from the first hour, first two hours, first three hours etc. and
run those
subsets through the analysis pipeline that was the intended application. This way they
only
needed to do one (longer) run.

## Install

**tl;dr**: precompiled binary

```shell
curl -sSL ontime.mbh.sh | sh
# or with wget
wget -nv -O - ontime.mbh.sh | sh
```

You can also pass options to the script like so

```
$ curl -sSL ontime.mbh.sh | sh -s -- --help
install.sh [option]

Fetch and install the latest version of ontime, if ontime is already
installed it will be updated to the latest version.

Options
        -V, --verbose
                Enable verbose output for the installer

        -f, -y, --force, --yes
                Skip the confirmation prompt during installation

        -p, --platform
                Override the platform identified by the installer [default: apple-darwin]

        -b, --bin-dir
                Override the bin installation directory [default: /usr/local/bin]

        -a, --arch
                Override the architecture identified by the installer [default: x86_64]

        -B, --base-url
                Override the base URL used for downloading releases [default: https://github.com/mbhall88/ssubmit/releases]

        -h, --help
                Display this help message
```

### Conda

[![Conda (channel only)](https://img.shields.io/conda/vn/bioconda/ontime)](https://anaconda.org/bioconda/ontime)
[![bioconda version](https://anaconda.org/bioconda/ontime/badges/platforms.svg)](https://anaconda.org/bioconda/ontime)
![Conda](https://img.shields.io/conda/dn/bioconda/ontime)

```shell
$ conda install -c bioconda ontime
```

### Cargo

![Crates.io](https://img.shields.io/crates/d/ontime)

```shell
$ cargo install ontime
```

### Container

Docker images are hosted at [quay.io].

#### `singularity`

Prerequisite: [`singularity`][singularity]

```shell
$ URI="docker://quay.io/mbhall88/ontime"
$ singularity exec "$URI" ontime --help
```

The above will use the latest version. If you want to specify a version then use a
[tag][quay.io] (or commit) like so.

```shell
$ VERSION="0.1.0"
$ URI="docker://quay.io/mbhall88/ontime:${VERSION}"
```

#### `docker`

[![Docker Repository on Quay](https://quay.io/repository/mbhall88/ontime/status "Docker Repository on Quay")](https://quay.io/repository/mbhall88/ontime)

Prerequisite: [`docker`][docker]

```shhell
$ docker pull quay.io/mbhall88/ontime
$ docker run quay.io/mbhall88/ontime ontime --help
```

You can find all the available tags on the [quay.io repository][quay.io].

### Build from source

```shell
$ git clone https://github.com/mbhall88/ontime.git
$ cd ontime
$ cargo build --release
$ target/release/ontime -h
```

## Examples

I want the reads that were sequenced **in the first hour**

```shell
$ ontime --to 1h in.fq
```

I want the reads that were sequenced **after the first hour**

```shell
$ ontime --from 1h in.fq
```

I want all reads **except those sequenced in the last hour**

```shell
$ ontime --to -1h in.fq
```

I want reads sequenced **between the third and fourth hours**

```shell
ontime --from 3h --to 4h in.fq
```

Check what the earliest and latest start times in the fastq are

```shell
$ ontime --show in.fq
Earliest: 2022-12-12T15:17:01.0Z
Latest  : 2022-12-13T01:16:27.0Z
```

I like to be specific, give me the reads that were sequenced **while I was eating dinner** (
see [note on time formats](#time-format))

```shell
ontime --from 2022-12-12T20:45:00Z --to 2022-12-12T21:17:01.5Z in.fq
```

I want to save the output to a Gzip-compressed file

```shell
$ ontime --to 2h -o out.fq.gz in.fq

```

## Usage

```
Usage: ontime [OPTIONS] <FILE>

Arguments:
  <FILE>  Input fastq file

Options:
  -o, --output <FILE>          Output file name [default: stdout]
  -O, --output-type <u|b|g|l>  u: uncompressed; b: Bzip2; g: Gzip; l: Lzma
  -L, --compress-level <1-21>  Compression level to use if compressing output [default: 6]
  -f, --from <DATE/DURATION>   Earliest start time; otherwise the earliest time is used
  -t, --to <DATE/DURATION>     Latest start time; otherwise the latest time is used
  -s, --show                   Show the earliest and latest start times in the input and exit
  -h, --help                   Print help (see more with '--help')
  -V, --version                Print version
```

#### Specifying a time range

The `--from` and `--to` options are used to restrict the timeframe you want reads from.
These options accept two different formats: duration and timestamp.

**Duration**: The most human-friendly way to provide a range is with duration. For
example, `1h` means 1 hour. Passing `--from 1h` says "I want reads that were generated 1
hour or more after sequencing started" - i.e. the earliest start time in the file plus 1
hour. Likewise, passing `--to 2h` says "I only want reads that were generated before the
second hour of sequencing". Using `--from` and `--to` in combination gives you a range.

We support a range of time/duration units and they can be combined. For example,
`3h45m` to indicate 3 hours and 45 minutes. See the [`duration-str` docs][duration] for
the full list
of support duration units.

Negative durations are also allowed. A negative duration subtracts that duration from
the **latest** start time in the file. So `--to -1h` will exclude reads that were
sequenced in the last hour of the run. Negative ranges are also valid -
i.e. `--from -2h --to -1h` will give you the reads sequenced in the penultimate hour of
the run.

**Timestamp**: If you want to provide date and time for your ranges, that is acceptable
in `--from/--to` also. See [the formatting guide](#time-format) for more information.

To make using timestamps a little easier, you can first run `ontime --show <in.fq>` to
get the earliest and latest timestamps in the file.

#### Time format

The times that `ontime` extracts are the `start_time=<time>` or `st:Z:<time>` section contained in the
description of each fastq read.
The format of this time has changed a few times, so if you come across a file
which `ontime` cannot parse, please raise an issue so I can make it work.

All times printed by `ontime` and accepted by the `--from/--to` options
are [UTC time][utc]. More recent versions of Guppy also have UTC offsets in
their `start_time`; for simplicity's sake, these offsets are ignored by `ontime`. So, if
you want to provide a timestamp to `--from/--to` based on a timeframe in your local
time, please first [convert it to UTC time][utc].

In general, the timestamp format `ontime` accepts anything that
is [RFC339-compliant][rfc3339].

The basic (recommended) format is `<YEAR>-<MONTH>-<DAY>T<HOUR>:<MINUTE>:<SECONDS>Z` -
e.g. `2022-12-12T18:39:09Z`. Feel free to get precise with
subseconds though if you like...

### Usage with Dorado output

[Dorado][dorado] (the latest basecaller from ONT) outputs BAM/SAM by default. If you want to use `ontime` on this data
you need to convert it to fastq/fasta, ensuring you keep the tags, as these contain the start time `ontime` relies on.

Convert BAM to fastq keeping all tags

```shell
$ samtools fastq -T '*' reads.bam > reads.fq
```

Or if you only want the start times

```shell
$ samtools fastq -T 'st' reads.bam > reads.fq
```

*Note: you can use `samtools fasta` instead if you only want FASTA output.*

The start time is encoded in the header of each read in the tag form `st:Z:<time>`.


### Full usage

```
Extract subsets of ONT (Nanopore) reads based on time

Usage: ontime [OPTIONS] <FILE>

Arguments:
  <FILE>
          Input fastq file

Options:
  -o, --output <FILE>
          Output file name [default: stdout]

  -O, --output-type <u|b|g|l>
          u: uncompressed; b: Bzip2; g: Gzip; l: Lzma

          ontime will attempt to infer the output compression format automatically from the output extension. If writing to stdout, the default is uncompressed (u)

  -L, --compress-level <1-21>
          Compression level to use if compressing output

          [default: 6]

  -f, --from <DATE/DURATION>
          Earliest start time; otherwise the earliest time is used

          This can be a timestamp - e.g. 2022-11-20T18:00:00 - or a duration from the start - e.g. 2h30m (2 hours and 30 minutes from the start). See the docs for more examples

  -t, --to <DATE/DURATION>
          Latest start time; otherwise the latest time is used

          See --from (and docs) for examples

  -s, --show
          Show the earliest and latest start times in the input and exit

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Cite

`ontime` is archived [at Zenodo](https://doi.org/10.5281/zenodo.7533052).

```bibtex
@software{ontime,
  author       = {Michael Hall},
  title        = {mbhall88/ontime: 0.1.3},
  month        = jan,
  year         = 2023,
  publisher    = {Zenodo},
  version      = {0.1.3},
  doi          = {10.5281/zenodo.7533053},
  url          = {https://doi.org/10.5281/zenodo.7533053}
}
```

[quay.io]: https://quay.io/repository/mbhall88/ontime

[singularity]: https://sylabs.io/guides/3.5/user-guide/quick_start.html#quick-installation-steps

[docker]: https://docs.docker.com/v17.12/install/

[utc]: https://www.timeanddate.com/worldclock/timezone/utc

[rfc3339]: https://www.rfc-editor.org/rfc/rfc3339#section-5.8

[duration]: https://github.com/baoyachi/duration-str#duration-unit-list

[dorado]: https://github.com/nanoporetech/dorado