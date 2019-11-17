/*
 * Copyright (c) 2010 Ant Kutschera, maxant
 *
 * The code below is free software: you can redistribute it and/or modify
 * it under the terms of the Lesser GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * The code in this file is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * Lesser GNU General Public License for more details.
 * You should have received a copy of the Lesser GNU General Public License
 * along with Foobar.  If not, see http://www.gnu.org/licenses/.
 */

package com.tokera.ate.common;

import java.math.BigInteger;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * allows you to convert a whole number into a compacted representation of that number,
 * based upon the dictionary you provide. very similar to base64 encoding, or indeed hex
 * encoding.
 */
public class BaseX {

    /**
     * contains hexadecimals 0-F only.
     */
    public static final char[] DICTIONARY_16 =
            new char[]{'0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F'};

    public static final char[] DICTIONARY_26 =
            new char[]{'a','b','c','d','e','f','g','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v','w','x','y','z'};

    public static final char[] DICTIONARY_36 =
            new char[]{'a','b','c','d','e','f','g','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v','w','x','y','z','0','1','2','3','4','5','6','7','8','9'};

    /**
     * contains only alphanumerics, in capitals and excludes letters/numbers which can be confused,
     * eg. 0 and O or L and I and 1.
     */
    public static final char[] DICTIONARY_32 =
            new char[]{'1','2','3','4','5','6','7','8','9','A','B','C','D','E','F','G','H','J','K','M','N','P','Q','R','S','T','U','V','W','X','Y','Z'};

    /**
     * contains only alphanumerics, including both capitals and smalls.
     */
    public static final char[] DICTIONARY_62 =
            new char[]{'0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F','G','H','I','J','K','L','M','N','O','P','Q','R','S','T','U','V','W','X','Y','Z','a','b','c','d','e','f','g','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v','w','x','y','z'};

    /**
     * contains alphanumerics, including both capitals and smalls, and the following special chars:
     * +"@*#%&/|()=?'~[!]{}-_:.,; (you might not be able to read all those using a browser!
     */
    public static final char[] DICTIONARY_89 =
            new char[]{'0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F','G','H','I','J','K','L','M','N','O','P','Q','R','S','T','U','V','W','X','Y','Z','a','b','c','d','e','f','g','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v','w','x','y','z','+','"','@','*','#','%','&','/','|','(',')','=','?','~','[',']','{','}','$','-','_','.',':',',',';','<','>'};

    protected char[] dictionary;

    /**
     * create an encoder with the given dictionary.
     *
     * @param dictionary the dictionary to use when encoding and decoding.
     */
    public BaseX(char[] dictionary){
        this.dictionary = dictionary;
    }

    /**
     * creates an encoder with the {@link #DICTIONARY_62} dictionary.
     */
    public BaseX(){
        this.dictionary = DICTIONARY_62;
    }

    /**
     * encodes the given string into the base of the dictionary provided in the constructor.
     * @param value the number to encode.
     * @return the encoded string.
     */
    public String encode(BigInteger value) {

        List<Character> result = new ArrayList<Character>();
        BigInteger base = new BigInteger("" + dictionary.length);
        int exponent = 1;
        BigInteger remaining = value;
        while(true){
            BigInteger a = base.pow(exponent); //16^1 = 16
            BigInteger b = remaining.mod(a); //119 % 16 = 7 | 112 % 256 = 112
            BigInteger c = base.pow(exponent - 1);
            BigInteger d = b.divide(c);

            //if d > dictionary.length, we have a problem. but BigInteger doesnt have
            //a greater than method :-(  hope for the best. theoretically, d is always
            //an index of the dictionary!
            result.add(dictionary[d.intValue()]);
            remaining = remaining.subtract(b); //119 - 7 = 112 | 112 - 112 = 0

            //finished?
            if(remaining.equals(BigInteger.ZERO)){
                break;
            }

            exponent++;
        }

        //need to reverse it, since the start of the list contains the least significant values
        StringBuffer sb = new StringBuffer();
        for(int i = result.size()-1; i >= 0; i--){
            sb.append(result.get(i));
        }
        return sb.toString();
    }

    /**
     * decodes the given string from the base of the dictionary provided in the constructor.
     * @param str the string to decode.
     * @return the decoded number.
     */
    public BigInteger decode(String str) {

        //reverse it, coz its already reversed!
        char[] chars = new char[str.length()];
        str.getChars(0, str.length(), chars, 0);

        char[] chars2 = new char[str.length()];
        int i = chars2.length -1;
        for(char c : chars){
            chars2[i--] = c;
        }

        //for efficiency, make a map
        Map<Character, BigInteger> dictMap = new HashMap<Character, BigInteger>();
        int j = 0;
        for(char c : dictionary){
            dictMap.put(c, new BigInteger("" + j++));
        }

        BigInteger bi = BigInteger.ZERO;
        BigInteger base = new BigInteger("" + dictionary.length);
        int exponent = 0;
        for(char c : chars2){
            BigInteger a = dictMap.get(c);
            BigInteger b = base.pow(exponent).multiply(a);
            bi = bi.add(new BigInteger("" + b));
            exponent++;
        }

        return bi;

    }

}