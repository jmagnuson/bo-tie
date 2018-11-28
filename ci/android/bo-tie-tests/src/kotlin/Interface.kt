package botie.testproject

class Interface {

  init {
    System.loadLibrary("bo_tie_tests");
  }

  external fun runTests(): String;
}
