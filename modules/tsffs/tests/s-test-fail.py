# Simplest possible failure test case, test that a failure can be checked for (test cases whose
# names end in -fail) are expected to fail.

from stest import fail

fail("Expected failure to test Rust test suite generation")
