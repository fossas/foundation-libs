# srclib

This library hosts types used in FOSSA that are "srclib" flavored format.

### History

`srclib` is a framework that FOSSA used _very_ early on, and its use early in
development has shaped FOSSA communication protocols such that most things,
from how we talk about dependency graphs to how we report licenses to even
the core way in which we talk about packages, was been influenced by how it worked.

At this point, `srclib` formatted communications are I think a shorthand
way to refer to a set of "legacy formats" that FOSSA is slowly in the
process of moving away from. Nevertheless, programs need to understand these
formats to work with the rest of FOSSA.

### Intent

Hence, this package: while other services or libraries may use other
communication formats for expressing FOSSA concepts, at the end of the day
the data must eventually collapse down to a `srclib`-compatible form
in order to be used by the rest of the FOSSA service ecosystem.

### Expectation

Given this, the expectation is that if a service exports data
that is meant to be consumed by some other FOSSA service,
that data needs to be converted (via this library) to a format that
other services can understand.

### Exceptions

Some FOSSA services do not speak `srclib` format, or do not always.
In such cases it is of course acceptable for a service that is intended
to only be consumed by a non-`srclib`-speaking service
to not provide `srclib` output. However it's recommended to always
provide `srclib` formatted output if posible, since it unlocks optionality.

It is broadly acceptable and desired for a service to provide both
`srclib`-formatted and non-`srclib`-formatted data representing the same
information, so long as the client can clearly choose between the two.
