package wit_numbers;

public class NumbersWorldImpl {
    private static void expect(boolean v) {
        if (!v) {
            throw new AssertionError();
        }
    }

    public static void testImports() {
        expect(Imports.roundtripU8((byte) 1) == (byte) 1);
        expect(Imports.roundtripU8((byte) 0) == (byte) 0);
        expect(Imports.roundtripU8((byte) 0xFF) == (byte) 0xFF);

        expect(Imports.roundtripS8((byte) 1) == (byte) 1);
        expect(Imports.roundtripS8(Byte.MIN_VALUE) == Byte.MIN_VALUE);
        expect(Imports.roundtripS8(Byte.MAX_VALUE) == Byte.MAX_VALUE);

        expect(Imports.roundtripU16((short) 1) == (short) 1);
        expect(Imports.roundtripU16((short) 0) == (short) 0);
        expect(Imports.roundtripU16((short) 0xFFFF) == (short) 0xFFFF);

        expect(Imports.roundtripS16((short) 1) == (short) 1);
        expect(Imports.roundtripS16(Short.MIN_VALUE) == Short.MIN_VALUE);
        expect(Imports.roundtripS16(Short.MAX_VALUE) == Short.MAX_VALUE);

        expect(Imports.roundtripU32(1) == 1);
        expect(Imports.roundtripU32(0) == 0);
        expect(Imports.roundtripU32(0xFFFFFFFF) == 0xFFFFFFFF);

        expect(Imports.roundtripS32(1) == 1);
        expect(Imports.roundtripS32(Integer.MIN_VALUE) == Integer.MIN_VALUE);
        expect(Imports.roundtripS32(Integer.MAX_VALUE) == Integer.MAX_VALUE);

        expect(Imports.roundtripU64(1L) == 1);
        expect(Imports.roundtripU64(0L) == 0L);
        expect(Imports.roundtripU64(0xFFFFFFFFFFFFFFFFL) == 0xFFFFFFFFFFFFFFFFL);

        expect(Imports.roundtripS64(1L) == 1L);
        expect(Imports.roundtripS64(Long.MIN_VALUE) == Long.MIN_VALUE);
        expect(Imports.roundtripS64(Long.MAX_VALUE) == Long.MAX_VALUE);

        expect(Imports.roundtripFloat32(1.0F) == 1.0F);
        expect(Imports.roundtripFloat32(Float.POSITIVE_INFINITY) == Float.POSITIVE_INFINITY);
        expect(Imports.roundtripFloat32(Float.NEGATIVE_INFINITY) == Float.NEGATIVE_INFINITY);
        expect(Float.isNaN(Imports.roundtripFloat32(Float.NaN)));

        expect(Imports.roundtripFloat64(1.0) == 1.0);
        expect(Imports.roundtripFloat64(Double.POSITIVE_INFINITY) == Double.POSITIVE_INFINITY);
        expect(Imports.roundtripFloat64(Double.NEGATIVE_INFINITY) == Double.NEGATIVE_INFINITY);
        expect(Double.isNaN(Imports.roundtripFloat64(Double.NaN)));

        expect(Imports.roundtripChar((int) 'a') == (int) 'a');
        expect(Imports.roundtripChar((int) ' ') == (int) ' ');
        expect(Imports.roundtripChar("🚩".codePointAt(0)) == "🚩".codePointAt(0));

        Imports.setScalar(2);
        expect(Imports.getScalar() == 2);
        Imports.setScalar(4);
        expect(Imports.getScalar() == 4);
    }

}
