# brown-noise

ブラウンノイズを再生するだけの、メニューバー（タスクトレイ）常駐アプリ。macOS / Windows / Linux 対応。

- 再生 / 停止、音量（25・50・75・100% のプリセットと ±10%）をメニューから操作
- 停止中はオーディオデバイスを解放し、CPU をほぼ使わない
- 出力デバイスが外れたら、再生中であれば現在の既定デバイスで再生し直す
- 音量と「起動時に再生」を保存し、「ログイン時に起動」を設定できる

## ビルドと実行

```sh
cargo run --release
```

### Linux の追加依存

トレイ表示に GTK 3 と AppIndicator が必要。

```sh
sudo apt install libgtk-3-dev libayatana-appindicator3-dev libasound2-dev
```

GNOME では AppIndicator 拡張を入れないとトレイが表示されない。

## 配布用パッケージ

### macOS

```sh
scripts/bundle-macos.sh
```

`target/release/bundle/Brown Noise.app` ができる。Dock には出ず、メニューバーにだけ常駐する。ad-hoc 署名のため、ほかの Mac に配る場合は Developer ID での署名と公証が要る。

### Windows / Linux

`cargo build --release` でできる `target/release/brown-noise(.exe)` をそのまま使う。Windows の release ビルドはコンソール窓を出さない。

## 設定ファイル

| OS | 場所 |
|---|---|
| macOS | `~/Library/Application Support/brown-noise/settings.conf` |
| Windows | `%APPDATA%\brown-noise\settings.conf` |
| Linux | `~/.config/brown-noise/settings.conf` |

「ログイン時に起動」は、そのとき動いている実行ファイルのパスを登録する。アプリを移動したら一度オフにしてからオンにし直す。

| OS | 登録先 |
|---|---|
| macOS | `~/Library/LaunchAgents/brown-noise.plist` |
| Windows | レジストリ `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` |
| Linux | `~/.config/autostart/brown-noise.desktop` |

## ライセンス

MIT
