echo
echo "Mounting your files at /usr..."
mkdir -p /usr
mount tok /usr $USER/usr
echo
echo "Mounting your data ate /opt..."
mkdir -p /opt
mount tok /opt $USER/opt
echo
echo "Welcome $USER,"
