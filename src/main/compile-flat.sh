FF=/prog/tokera/tokapi/flatbuffers

pushd java 1>/dev/null
$FF/flatc --java ../resources/schema/msg/base.fbs
$FF/flatc --java ../resources/schema/msg/data.fbs
$FF/flatc --java ../resources/schema/msg/key.fbs
pushd com/tokera/ate/dao/msg 1>/dev/null
sed -i "s/@SuppressWarnings(\"unused\")/@SuppressWarnings({\"unused\", \"return.type.incompatible\"})/g" *.java
sed -i "s/  public String /  public @org.checkerframework.checker.nullness.qual.Nullable String /g" *.java
sed -i "s/  public ByteBuffer /  public @org.checkerframework.checker.nullness.qual.Nullable ByteBuffer /g" *.java
sed -i "s/  public MessageDataHeader /  public @org.checkerframework.checker.nullness.qual.Nullable MessageDataHeader /g" *.java
sed -i "s/  public MessageDataDigest /  public @org.checkerframework.checker.nullness.qual.Nullable MessageDataDigest /g" *.java
sed -i "s/  public Table /  public @org.checkerframework.checker.nullness.qual.Nullable Table /g" *.java
popd 1>/dev/null
popd 1>/dev/null

pushd cpp 1>/dev/null
$FF/flatc --cpp ../resources/schema/common.fbs
$FF/flatc --cpp ../resources/schema/msg/base.fbs
$FF/flatc --cpp ../resources/schema/msg/data.fbs
$FF/flatc --cpp ../resources/schema/msg/encrypt.text.fbs
$FF/flatc --cpp ../resources/schema/msg/key.fbs
popd 1>/dev/null
