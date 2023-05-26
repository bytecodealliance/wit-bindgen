package wit.worlds;

import wit.imports.test.numbers.Test;

public class NumbersImpl {
    private static void expect(boolean v) {
        if (!v) {
            throw new AssertionError();
        }
    }

    public static void testImports() {
        expect(Test.roundtripU8((byte) 1) == (byte) 1);
        expect(Test.roundtripU8((byte) 0) == (byte) 0);
        expect(Test.roundtripU8((byte) 0xFF) == (byte) 0xFF);

        expect(Test.roundtripS8((byte) 1) == (byte) 1);
        expect(Test.roundtripS8(Byte.MIN_VALUE) == Byte.MIN_VALUE);
        expect(Test.roundtripS8(Byte.MAX_VALUE) == Byte.MAX_VALUE);

        expect(Test.roundtripU16((short) 1) == (short) 1);
        expect(Test.roundtripU16((short) 0) == (short) 0);
        expect(Test.roundtripU16((short) 0xFFFF) == (short) 0xFFFF);

        expect(Test.roundtripS16((short) 1) == (short) 1);
        expect(Test.roundtripS16(Short.MIN_VALUE) == Short.MIN_VALUE);
        expect(Test.roundtripS16(Short.MAX_VALUE) == Short.MAX_VALUE);

        expect(Test.roundtripU32(1) == 1);
        expect(Test.roundtripU32(0) == 0);
        expect(Test.roundtripU32(0xFFFFFFFF) == 0xFFFFFFFF);

        expect(Test.roundtripS32(1) == 1);
        expect(Test.roundtripS32(Integer.MIN_VALUE) == Integer.MIN_VALUE);
        expect(Test.roundtripS32(Integer.MAX_VALUE) == Integer.MAX_VALUE);

        expect(Test.roundtripU64(1L) == 1);
        expect(Test.roundtripU64(0L) == 0L);
        expect(Test.roundtripU64(0xFFFFFFFFFFFFFFFFL) == 0xFFFFFFFFFFFFFFFFL);

        expect(Test.roundtripS64(1L) == 1L);
        expect(Test.roundtripS64(Long.MIN_VALUE) == Long.MIN_VALUE);
        expect(Test.roundtripS64(Long.MAX_VALUE) == Long.MAX_VALUE);

        expect(Test.roundtripFloat32(1.0F) == 1.0F);
        expect(Test.roundtripFloat32(Float.POSITIVE_INFINITY) == Float.POSITIVE_INFINITY);
        expect(Test.roundtripFloat32(Float.NEGATIVE_INFINITY) == Float.NEGATIVE_INFINITY);
        expect(Float.isNaN(Test.roundtripFloat32(Float.NaN)));

        expect(Test.roundtripFloat64(1.0) == 1.0);
        expect(Test.roundtripFloat64(Double.POSITIVE_INFINITY) == Double.POSITIVE_INFINITY);
        expect(Test.roundtripFloat64(Double.NEGATIVE_INFINITY) == Double.NEGATIVE_INFINITY);
        expect(Double.isNaN(Test.roundtripFloat64(Double.NaN)));

        expect(Test.roundtripChar((int) 'a') == (int) 'a');
        expect(Test.roundtripChar((int) ' ') == (int) ' ');
        expect(Test.roundtripChar("🚩".codePointAt(0)) == "🚩".codePointAt(0));

        Test.setScalar(2);
        expect(Test.getScalar() == 2);
        Test.setScalar(4);
        expect(Test.getScalar() == 4);
    }

}
