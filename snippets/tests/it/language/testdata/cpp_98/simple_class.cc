namespace ns {

  class Foo {
    int another_method();
    
    int member_method(){
      // This is defined inside the class def.
      return 0;
    }
  };

  int Foo::another_method() {
    /* This method is from outside the class def */
    return 0;
  }

  int a_bare_function(/* signature comment */  ){
    // This function is not part of a class at all
    return 0;
  }
}
