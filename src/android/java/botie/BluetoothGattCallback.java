package botie;

import android.bluetooth.BluetoothGatt;
import android.bluetooth.BluetoothGattCharacteristic;
import android.bluetooth.BluetoothGattDescriptor;

public class BluetoothGattCallback extends android.bluetooth.BluetoothGattCallback {

  @Override
  public native void onCharacteristicChanged(BluetoothGatt gatt, BluetoothGattCharacteristic characteristic);

  @Override
  public native void onCharacteristicRead(BluetoothGatt gatt, BluetoothGattCharacteristic characteristic, int status);

  @Override
  public native void onCharacteristicWrite(BluetoothGatt gatt, BluetoothGattCharacteristic characteristic, int status);

  @Override
  public native void onConnectionStateChange(BluetoothGatt gatt, int status, int newState);

  @Override
  public native void onDescriptorRead(BluetoothGatt gatt, BluetoothGattDescriptor descriptor, int status);

  @Override
  public native void onDescriptorWrite(BluetoothGatt gatt, BluetoothGattDescriptor descriptor, int status);

  @Override
  public native void onMtuChanged(BluetoothGatt gatt, int mtu, int status);

  @Override
  public native void onPhyRead(BluetoothGatt gatt, int txPhy, int rxPhy, int status);

  @Override
  public native void onPhyUpdate(BluetoothGatt gatt, int txPhy, int rxPhy, int status);

  @Override
  public native void onReadRemoteRssi(BluetoothGatt gatt, int rssi, int status);

  @Override
  public native void onReliableWriteCompleted(BluetoothGatt gatt, int status);

  @Override
  public native void onServicesDiscovered(BluetoothGatt gatt, int status);

  @Override
  protected void finalize() throws Throwable {
    super.finalize();
    cleanBotie();
  }

  private native void cleanBotie();
}
