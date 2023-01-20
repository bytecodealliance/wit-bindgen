package wit_numbers;

public class ExportsImpl {
    public static byte roundtripU8(byte a) {
        return a;
    }

    public static byte roundtripS8(byte a) {
        return a;
    }

    public static short roundtripU16(short a) {
        return a;
    }

    public static short roundtripS16(short a) {
        return a;
    }

    public static int roundtripU32(int a) {
        return a;
    }

    public static int roundtripS32(int a) {
        return a;
    }

    public static long roundtripU64(long a) {
        return a;
    }

    public static long roundtripS64(long a) {
        return a;
    }

    public static float roundtripFloat32(float a) {
        return a;
    }

    public static double roundtripFloat64(double a) {
        return a;
    }

    public static int roundtripChar(int a) {
        return a;
    }

    private static int scalar = 0;

    public static void setScalar(int a) {
        scalar = a;
    }

    public static int getScalar() {
        return scalar;
    }
}
