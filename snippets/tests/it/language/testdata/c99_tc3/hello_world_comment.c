
#include <stdio.h>

// Just a little comment. For fun.

/* indicates error code */ int /* the classic starting point */ main /* who needs params? */ (/* no params, because let's keep this "simple" */) // and end with a bang
{ // Some hello world, huh?
  // what if we could never decide on a comment style?

  /**
* // Fortunately, GCC itself dies on nested multiline comments
    *    // So at least we don't have to worry about that...
  */// But we do need to make sure to handle single line after multiline
  printf/* () */("hello world\n" /* ) */ ); // the classic beginning

/* comments can be everywhere! */ return /* why not between syntax? */ 0 /* go crazy!*/;

  /*
    An ode to the return 0
    
    O Return Zero, O humble sign,
    At end of code, you draw the line.
    The program's close, the work is done,
    A battle fought, a victory won.
    
    You tell the world, "It's all okay,
    My code has run, now let's away!"
    In silent grace, you make your stand,
    A sentinel in a shifting land.
    
    No errors here, no bugs in sight,
    Just zeros glowing in the night.
    In digit small, the tale's been told,
    A hero's end, forever bold.
    
    And though we part, you'll always be,
    A zero full of meaning, see?
    O Return Zero, to you I toast,
    For ending well, when matters most.
  */ } // Yes, recoil in horror

// This does compile, yes.
