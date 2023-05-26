package wit.worlds;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.ArrayList;

import wit.imports.test.lists.Test;
import wit.worlds.Lists.Tuple2;

public class ListsImpl {
    public static int allocatedBytes() {
        return 0;
    }

    public static void testImports() {
        Test.emptyListParam(new byte[0]);

        Test.emptyStringParam("");

        {
            byte[] result = Test.emptyListResult();
            expect(result.length == 0);
        }

        {
            String result = Test.emptyStringResult();
            expect(result.length() == 0);
        }

        Test.listParam(new byte[] { (byte) 1, (byte) 2, (byte) 3, (byte) 4 });

        Test.listParam2("foo");

        Test.listParam3(new ArrayList<String>() {{
            add("foo");
            add("bar");
            add("baz");
        }});

        Test.listParam4(new ArrayList<ArrayList<String>>() {{
            add(new ArrayList<String>() {{
                add("foo");
                add("bar");
            }});
            add(new ArrayList<String>() {{
                add("baz");
            }});
        }});

        {
            byte[] result = Test.listResult();
            expect(result.length == 5);
            expect(result[0] == (byte) 1);
            expect(result[1] == (byte) 2);
            expect(result[2] == (byte) 3);
            expect(result[3] == (byte) 4);
            expect(result[4] == (byte) 5);
        }

        {
            String result = Test.listResult2();
            expect(result.equals("hello!"));
        }

        {
            ArrayList<String> result = Test.listResult3();
            expect(result.size() == 2);
            expect(result.get(0).equals("hello,"));
            expect(result.get(1).equals("world!"));
        }

        for (String s : new String[] { "x", "", "hello", "hello ⚑ world" }) {
            String result = Test.stringRoundtrip(s);
            expect(result.equals(s));

            byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
            expect(Arrays.equals(bytes, Test.listRoundtrip(bytes)));
        }

        {
            Tuple2<byte[], byte[]> result = Test.listMinmax8
                (new byte[] { (byte) 0, (byte) 0xFF }, new byte[] { (byte) 0x80, (byte) 0x7F });

            expect(result.f0.length == 2 && result.f0[0] == (byte) 0 && result.f0[1] == (byte) 0xFF);
            expect(result.f1.length == 2 && result.f1[0] == (byte) 0x80 && result.f1[1] == (byte) 0x7F);
        }

        {
            Tuple2<short[], short[]> result = Test.listMinmax16
                (new short[] { (short) 0, (short) 0xFFFF }, new short[] { (short) 0x8000, (short) 0x7FFF });

            expect(result.f0.length == 2 && result.f0[0] == (short) 0 && result.f0[1] == (short) 0xFFFF);
            expect(result.f1.length == 2 && result.f1[0] == (short) 0x8000 && result.f1[1] == (short) 0x7FFF);
        }

        {
            Tuple2<int[], int[]> result = Test.listMinmax32
                (new int[] { 0, 0xFFFFFFFF }, new int[] { 0x80000000, 0x7FFFFFFF });

            expect(result.f0.length == 2 && result.f0[0] == 0 && result.f0[1] == 0xFFFFFFFF);
            expect(result.f1.length == 2 && result.f1[0] == 0x80000000 && result.f1[1] == 0x7FFFFFFF);
        }

        {
            Tuple2<long[], long[]> result = Test.listMinmax64
                (new long[] { 0, 0xFFFFFFFFFFFFFFFFL }, new long[] { 0x8000000000000000L, 0x7FFFFFFFFFFFFFFFL });

            expect(result.f0.length == 2
                   && result.f0[0] == 0
                   && result.f0[1] == 0xFFFFFFFFFFFFFFFFL);

            expect(result.f1.length == 2
                   && result.f1[0] == 0x8000000000000000L
                   && result.f1[1] == 0x7FFFFFFFFFFFFFFFL);
        }

        {
            Tuple2<float[], double[]> result = Test.listMinmaxFloat
                (new float[] {
                    -Float.MAX_VALUE,
                    Float.MAX_VALUE,
                    Float.NEGATIVE_INFINITY,
                    Float.POSITIVE_INFINITY
                },
                    new double[] {
                        -Double.MAX_VALUE,
                        Double.MAX_VALUE,
                        Double.NEGATIVE_INFINITY,
                        Double.POSITIVE_INFINITY
                    });

            expect(result.f0.length == 4
                   && result.f0[0] == -Float.MAX_VALUE
                   && result.f0[1] == Float.MAX_VALUE
                   && result.f0[2] == Float.NEGATIVE_INFINITY
                   && result.f0[3] == Float.POSITIVE_INFINITY);

            expect(result.f1.length == 4
                   && result.f1[0] == -Double.MAX_VALUE
                   && result.f1[1] == Double.MAX_VALUE
                   && result.f1[2] == Double.NEGATIVE_INFINITY
                   && result.f1[3] == Double.POSITIVE_INFINITY);
        }
    }

    public static void expect(boolean v) {
        if (!v) {
            throw new AssertionError();
        }
    }
}
