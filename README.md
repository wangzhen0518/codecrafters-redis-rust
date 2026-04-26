# TODO

1. id 分配问题，如何快速得到可用的 id？
2. rdb
3. 重构，提升性能
   1. lexcial_write 函数，在 lexical_write 之前，能否只调整 BytesMut 的大小，而不预填充 0？从而写完之后也不需要 truncate
   2. 不复制 input buffer
   3. 重构 resp 和 command 的 parse、serialize、execution 的逻辑
   4. parse、serialize、execution 过程中减少 bytes 和 &str 的复制
