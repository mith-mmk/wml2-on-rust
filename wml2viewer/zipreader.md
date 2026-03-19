# zip createがネットワーク共有に弱い問題
- Read + Seek 前提 Reader 
- ZIP（ランダムアクセス前提）
- ネットワーク（高レイテンシ）

この問題により、固まる。

## 対応策
Readerの抽象を分ける

```rs
trait RandomAccessReader {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]);
}
```

間にキャッシュを挟む
- file reader
- cache reader
- remote reader


## 最低限の実装
-  末尾キャッシュ（必須）
- Central Directory用
- チャンクキャッシュ
例: 1MB単位
LRU or simple ring buffer
実装イメージ
```rs
struct ZipCachedReader<R> {
    inner: R,
    cache: HashMap<u64, Vec<u8>>, // chunk単位
    chunk_size: u64,
}
fn read_at(&mut self, offset: u64, buf: &mut [u8]) {
    // chunk単位に分解してキャッシュヒット確認
}
```

## ZIP側の改善ポイント
- EOCD探索を最適化

bad code
```rs
for i in 0.. {
    seek(end - i)
    read(1)
}

- 修正
```rs
// 一発で後ろ数MB読む
seek(end - N)
read(N)
```

memchrでEOCD探す
Central Directoryもまとめ読み

[重要] 1件ずつreadしない

## 最終対応策
ローカル前提（高速）
```rs
ZipArchive<File>
```

ネットワーク対応
```rs
ZipArchive<ZipCachedReader<RemoteReader>>
```

- インデックスだけソートして遅延アクセス

# まとめ

- Reader抽象を分離
- キャッシュレイヤを必須化
- EOCDを一括読み
- Central Directoryをまとめ読み
- ソートは諦める or 遅延
