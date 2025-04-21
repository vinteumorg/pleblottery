<h1 align="center">
  <br>
  <img width="700" src="src/web/assets/images/pleblottery.png">
<br>
</h1>

<p align="center">
⛏️ plebs be hashin ⚡
</p>

## intro

`pleblottery` is a Rust-based hashrate aggregator for a pleb-friendly and fully sovereign solo/lottery Bitcoin mining experience over [Stratum V2](https://stratumprotocol.org).

the idea is similar to [`public-pool`](https://github.com/benjamin-wilson/public-pool) and [`ckpool-solo`](https://bitbucket.org/ckolivas/ckpool-solo/), but we're explicitly avoiding the "pool" terminology to avoid ambiguity and confusion.

the coinbase payout goes to one single output, **without any kind of pooled reward distribution**.

`pleblottery` builds on top of [`tower-stratum`](https://github.com/plebhash/tower-stratum) and [Stratum V2 Reference Implementation](https://github.com/stratum-mining/stratum).


<h1 align="center">
  <br>
  <img width="700" src="diagram.png">
<br>
</h1>

## `flake`-based `pleblottery-playground`

`flake.nix` deploys `pleblottery-playground` environment, which consists of:
- Sv2 Template Provider ([Bitcoin Core Sv2 Patch by Sjors](https://github.com/Sjors/bitcoin)) connected to a custom signet of Sv2 Community.
- `pleblottery` instance

in order to launch `pleblottery-playground`, run:

```
$ nix develop
# Building Bitcoin Core Sv2 fork by @Sjors via nix-bitcoin-core-archive...
/nix/store/8j3apdyyg4lanki4f4mabc3yl0w0lf20-bitcoind-sv2-v28.99.0
Creating bitcoin.conf...

Environment setup complete!
Run 'pleblottery_playground' to launch bitcoind and pleblottery side-by-side in tmux
# pleblottery_playground
```

## mainnet instructions

soon™
