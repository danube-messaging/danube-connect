#!/bin/bash
# Test script to send embeddings to Danube using danube-cli

set -e

# Configuration
DANUBE_URL=${DANUBE_URL:-http://localhost:6650}
TOPIC=${TOPIC:-/default/vectors}
EMBEDDINGS_FILE=${EMBEDDINGS_FILE:-embeddings.jsonl}
DANUBE_CLI=${DANUBE_CLI:-danube-cli}
INTERVAL=${INTERVAL:-500}

echo "=" | tr '=' '\n' | head -c 60 && echo
echo "📤 Danube Producer for Qdrant Sink Connector"
echo "=" | tr '=' '\n' | head -c 60 && echo
echo "Danube URL: ${DANUBE_URL}"
echo "Topic: ${TOPIC}"
echo "Input File: ${EMBEDDINGS_FILE}"
echo "Interval: ${INTERVAL}ms"
echo ""

# Check if embeddings file exists
if [ ! -f "${EMBEDDINGS_FILE}" ]; then
    echo "❌ Error: Embeddings file not found: ${EMBEDDINGS_FILE}"
    echo ""
    echo "💡 Generate embeddings first:"
    echo "   python3 generate_embeddings.py --count 10"
    echo ""
    exit 1
fi

# Check if danube-cli is available
if ! command -v ${DANUBE_CLI} &> /dev/null; then
    echo "❌ Error: danube-cli not found"
    echo ""
    echo "💡 Build danube-cli:"
    echo "   git clone https://github.com/danrusei/danube.git"
    echo "   cd danube/danube-cli"
    echo "   cargo build --release"
    echo "   export PATH=\$PATH:\$(pwd)/target/release"
    echo ""
    echo "Or specify path:"
    echo "   DANUBE_CLI=/path/to/danube-cli ./test_producer.sh"
    echo ""
    exit 1
fi

# Count total messages
total=$(wc -l < "${EMBEDDINGS_FILE}")
echo "📊 Found ${total} messages in ${EMBEDDINGS_FILE}"
echo ""

# Check Danube connectivity
echo "🔍 Checking Danube connectivity..."
if ! curl -s -f "${DANUBE_URL}/health" > /dev/null 2>&1; then
    echo "⚠️  Warning: Cannot reach Danube at ${DANUBE_URL}"
    echo "   Make sure Danube broker is running"
    echo ""
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "✅ Ready to send messages"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Send messages
count=0
success=0
failed=0

while IFS= read -r message; do
    count=$((count + 1))
    
    # Extract just the text for display (if it exists)
    text=$(echo "$message" | jq -r '.payload.text // "N/A"' 2>/dev/null || echo "N/A")
    text_short="${text:0:50}"
    
    # Send message using danube-cli
    if echo "$message" | ${DANUBE_CLI} produce \
        --service-addr "${DANUBE_URL}" \
        --topic "${TOPIC}" \
        --schema json \
        --message "$message" \
        --interval ${INTERVAL} \
        > /dev/null 2>&1; then
        success=$((success + 1))
        echo "✅ [${count}/${total}] Sent: ${text_short}..."
    else
        failed=$((failed + 1))
        echo "❌ [${count}/${total}] Failed: ${text_short}..."
    fi
    
    # Small delay between messages
    sleep 0.1
    
done < "${EMBEDDINGS_FILE}"

# Summary
echo ""
echo "=" | tr '=' '\n' | head -c 60 && echo
echo "📊 Summary"
echo "=" | tr '=' '\n' | head -c 60 && echo
echo "Total: ${total}"
echo "Success: ${success}"
echo "Failed: ${failed}"
echo "=" | tr '=' '\n' | head -c 60 && echo

if [ ${success} -gt 0 ]; then
    echo ""
    echo "💡 Next steps:"
    echo "   1. Check connector logs:"
    echo "      docker-compose logs -f qdrant-sink"
    echo ""
    echo "   2. View Qdrant dashboard:"
    echo "      http://localhost:6333/dashboard"
    echo ""
    echo "   3. Search vectors:"
    echo "      ./search_vectors.py --query 'password reset'"
    echo ""
fi
