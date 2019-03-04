package botie;

public class AdvertiseCallback extends android.bluetooth.le.AdvertiseCallback {

	@Override
	public native void onStartFailure(int errorCode);

	@Override
	public native void onStartSuccess(android.bluetooth.le.AdvertiseSettings settingsInEffect);

	@Override
	protected void finalize() throws Throwable {
		super.finalize();
		cleanBotie();
	}

	private native void cleanBotie();
}
