package botie;

public class ScanCallback extends android.bluetooth.le.ScanCallback {

  @Override
  public native void onBatchScanResults(java.util.List<android.bluetooth.le.ScanResult> results);

  @Override
  public native void onScanFailed(int errorCode);

  @Override
  public native void onScanResult(int callbackType, android.bluetooth.le.ScanResult result);

  @Override
  protected void finalize() throws Throwable {
    super.finalize();
    cleanBotie();
  }
  
  private native void cleanBotie();
}
