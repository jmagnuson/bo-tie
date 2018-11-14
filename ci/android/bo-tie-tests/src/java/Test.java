class Test {
  private static native void testInit();
  private static native void runTests();

  static {
    System.loadLibrary("botietests");
  }

  public static void main(String[] args) {
    testInit();
    runTest();
  }
}
