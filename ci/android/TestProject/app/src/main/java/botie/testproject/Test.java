
class Test {
  private static native String runTests();

  static {
    System.loadLibrary("botietests");
  }

  public static void main(String[] args) {
    runTests();
  }
}
