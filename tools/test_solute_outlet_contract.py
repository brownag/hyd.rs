"""Exact small equations for the reviewed solute outlet contract.

These are hand-entered rational systems. The checker verifies their explicit
solutions and balances; it does not assemble or solve a transport model.
"""

from fractions import Fraction as F
import unittest


def row_value(row, values):
    return sum((coefficient * value for coefficient, value in zip(row, values)), F(0))


def assert_equations(test, matrix, values, rhs):
    test.assertEqual(tuple(row_value(row, values) for row in matrix), tuple(rhs))


def inventory(widths, theta, concentrations, capacity=F(1)):
    return sum(
        (width * water * concentration * capacity
         for width, water, concentration in zip(widths, theta, concentrations)),
        F(0),
    )


class OutletContractTests(unittest.TestCase):
    def test_lower_outlet_nonuniform_with_interior_dispersion(self):
        # Uniform spacing 2 gives half-cell widths (1, 2, 1); theta=1/2.
        # Interior q=-1/2 and diffusive conductance g=1/4 on each face
        # (spacing 2, theta_face=1/2, effective dispersion D=1). Lower
        # outflow is 1/2.
        matrix = (
            (F(5, 4), F(-3, 4), F(0)),
            (F(-1, 4), F(2), F(-3, 4)),
            (F(0), F(-1, 4), F(5, 4)),
        )
        new = (F(1), F(1, 2), F(1, 4))
        rhs = (F(7, 8), F(9, 16), F(3, 16))
        assert_equations(self, matrix, new, rhs)

        old = (F(7, 4), F(9, 16), F(3, 8))
        widths = (F(1), F(2), F(1))
        theta = (F(1, 2),) * 3
        old_inventory = inventory(widths, theta, old)
        new_inventory = inventory(widths, theta, new)
        lower_exchange = F(-1, 2) * new[0]
        self.assertEqual(old_inventory, F(13, 8))
        self.assertEqual(new_inventory, F(9, 8))
        self.assertEqual(new_inventory - old_inventory, lower_exchange)
        self.assertEqual(lower_exchange, F(-1, 2))
        # Nonzero D is an interior-face term; the outlet contributes no D term.

    def test_upper_outlet_mirrored_nonuniform_case(self):
        matrix = (
            (F(5, 4), F(-1, 4), F(0)),
            (F(-3, 4), F(2), F(-1, 4)),
            (F(0), F(-3, 4), F(5, 4)),
        )
        new = (F(1, 4), F(1, 2), F(1))
        rhs = (F(3, 16), F(9, 16), F(7, 8))
        assert_equations(self, matrix, new, rhs)

        old = (F(3, 8), F(9, 16), F(7, 4))
        widths = (F(1), F(2), F(1))
        theta = (F(1, 2),) * 3
        old_inventory = inventory(widths, theta, old)
        new_inventory = inventory(widths, theta, new)
        upper_exchange = -F(1, 2) * new[2]
        self.assertEqual(old_inventory, F(13, 8))
        self.assertEqual(new_inventory, F(9, 8))
        self.assertEqual(new_inventory - old_inventory, upper_exchange)
        self.assertEqual(upper_exchange, F(-1, 2))

    def test_zero_flow_has_zero_exchange(self):
        # With q=0, no dispersion, and no reaction, the diagonal storage rows
        # leave each concentration unchanged; either endpoint mode is admissible.
        old = (F(2), F(1, 2), F(3, 4))
        widths = (F(1), F(2), F(1))
        theta = (F(1, 2),) * 3
        diagonal_storage = (F(1, 2), F(1), F(1, 2))
        matrix = tuple(
            tuple(diagonal_storage[i] if i == j else F(0) for j in range(3))
            for i in range(3)
        )
        rhs = tuple(diagonal_storage[i] * old[i] for i in range(3))
        assert_equations(self, matrix, old, rhs)
        exchange = F(0) * old[0] - F(0) * old[2]
        self.assertEqual(exchange, 0)
        self.assertEqual(inventory(widths, theta, old) - inventory(widths, theta, old), 0)

    def test_linear_sorption_decay_and_outlet_close_independently(self):
        # Two unit-width control volumes, theta=1/2, rho=1, Kd=1:
        # local stored mass is (theta + rho*Kd)*C = 3/2 C. dt=1,
        # first-order decay=1/2, q_lower=-1/2, and the interior q,D are zero.
        matrix = ((F(11, 4), F(0)), (F(0), F(9, 4)))
        new = (F(2), F(1))
        rhs = (F(11, 2), F(9, 4))
        assert_equations(self, matrix, new, rhs)
        old = (F(11, 3), F(3, 2))

        old_storage = F(3, 2) * sum(old, F(0))
        new_storage = F(3, 2) * sum(new, F(0))
        boundary_exchange = F(-1, 2) * new[0]
        reaction_change = -F(1, 2) * new_storage
        self.assertEqual(old_storage, F(31, 4))
        self.assertEqual(new_storage, F(9, 2))
        self.assertEqual(boundary_exchange, F(-1))
        self.assertEqual(reaction_change, F(-9, 4))
        self.assertEqual(new_storage - old_storage, boundary_exchange + reaction_change)

    def test_direct_source_row_replacement_is_a_different_discrete_system(self):
        # Starting from the lower-outlet finite-volume equations above, replace
        # row 0 by the pinned source's -C0+C1=0 equality row, leaving the other
        # candidate rows unchanged. This isolated row-replacement diagnostic is
        # not a claim that the complete Fortran stencil equals this matrix.
        source_row_matrix = (
            (F(-1), F(1), F(0)),
            (F(-1, 4), F(2), F(-3, 4)),
            (F(0), F(-1, 4), F(5, 4)),
        )
        source_row_solution = (F(27, 64), F(27, 64), F(15, 64))
        rhs = (F(0), F(9, 16), F(3, 16))
        assert_equations(self, source_row_matrix, source_row_solution, rhs)

        widths = (F(1), F(2), F(1))
        theta = (F(1, 2),) * 3
        source_inventory = inventory(widths, theta, source_row_solution)
        finite_volume_inventory = F(9, 8)
        source_row_exchange = F(-1, 2) * source_row_solution[0]
        old_inventory = F(13, 8)
        self.assertEqual(source_inventory, F(3, 4))
        self.assertEqual(source_inventory - old_inventory, F(-7, 8))
        self.assertEqual(source_row_exchange, F(-27, 128))
        self.assertEqual(finite_volume_inventory, F(9, 8))
        self.assertNotEqual(source_inventory, finite_volume_inventory)
        self.assertNotEqual(source_inventory - old_inventory, source_row_exchange)

    def test_inward_flow_domain_at_both_endpoints(self):
        # Signed q is positive toward increasing x. Inward means q>0 at the
        # lower endpoint and q<0 at the upper endpoint; exact zero is accepted.
        def admissible(endpoint, q):
            return q <= 0 if endpoint == "lower" else q >= 0

        self.assertFalse(admissible("lower", F(1, 10)))
        self.assertTrue(admissible("lower", F(0)))
        self.assertTrue(admissible("lower", F(-1, 10)))
        self.assertTrue(admissible("upper", F(1, 10)))
        self.assertTrue(admissible("upper", F(0)))
        self.assertFalse(admissible("upper", F(-1, 10)))


if __name__ == "__main__":
    unittest.main()
