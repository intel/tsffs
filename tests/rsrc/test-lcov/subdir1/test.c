#include <stdio.h>
extern int x(int);

int main() {
  int a = 0;

  a += 1;

  a = x(a);

  if (a == 15) {
    printf("%s\n", "hello");
  } else {
    printf("no\n");
  }
}
