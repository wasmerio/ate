package com.tokera.ate.common;

import java.io.IOException;
import java.io.StringReader;

import com.tokera.ate.annotations.StartupScoped;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.w3c.dom.Element;
import org.w3c.dom.Node;
import org.w3c.dom.NodeList;

import javax.ws.rs.WebApplicationException;
import javax.xml.transform.Transformer;
import javax.xml.transform.TransformerException;
import javax.xml.transform.TransformerFactory;
import javax.xml.transform.dom.DOMSource;
import javax.xml.transform.stream.StreamResult;
import java.io.StringWriter;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.xml.transform.OutputKeys;

import org.w3c.dom.Document;

import javax.xml.parsers.DocumentBuilder;
import javax.xml.parsers.DocumentBuilderFactory;
import javax.xml.parsers.ParserConfigurationException;
import org.xml.sax.InputSource;

/**
 * Helper class used on XML documents. Primarily this class is used to parse and pretty print them
 * @author jonhanlee
 */
@StartupScoped
@ApplicationScoped
public class XmlUtils {

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    public @Nullable Element getFirstChildElement(Element parentElement, String tagName) {
        NodeList nl = parentElement.getChildNodes();
        for (int i = 0; i < nl.getLength(); i++) {
            Node thisNode = nl.item(i);
            if (thisNode instanceof Element) {
                Element thisElement = (Element) thisNode;
                if (thisElement.getTagName().equals(tagName)) {
                    return thisElement;
                }
            }
        }
        return null;
    }
    
    public String prettyPrint(String unformattedXml) {
        try {
            final Document doc = parseXml(unformattedXml);
            
            Transformer transformer = TransformerFactory.newInstance().newTransformer();
            transformer.setOutputProperty(OutputKeys.INDENT, "yes");
            transformer.setOutputProperty("{http://xml.apache.org/xslt}indent-amount", "2");
            //initialize StreamResult with File object to save to file
            StreamResult result = new StreamResult(new StringWriter());
            DOMSource source = new DOMSource(doc);
            transformer.transform(source, result);
            return result.getWriter().toString();
        } catch (TransformerException ex) {
            throw new WebApplicationException(ex);
        }
    }
    
    public Document parseXml(String in) {
        try {
            DocumentBuilderFactory dbf = DocumentBuilderFactory.newInstance();
            DocumentBuilder db = dbf.newDocumentBuilder();
            InputSource is = new InputSource(new StringReader(in));
            return db.parse(is);
        } catch (ParserConfigurationException | IOException | org.xml.sax.SAXException ex) {
            throw new WebApplicationException(ex);
        }
    }
}
