import androidx.test.runner.AndroidJUnit4;

@LargeTest
public class android-testInstrumentTests {

    public native void myBasicFunction();

    @Test
    public void myBasicFunctionUnitTest() {
        myBasicFunction();
    }

    //<{Method hook for crate android-test. Unfortunately I don't get removed, so please just ignore or delete me :P }>

}
