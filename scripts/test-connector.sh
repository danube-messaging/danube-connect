#!/bin/bash
# Universal connector test script
# Usage: ./scripts/test-connector.sh <connector-name> [command]
#
# Commands:
#   start   - Start the example (default)
#   stop    - Stop the example
#   logs    - Show connector logs
#   restart - Restart the example
#   test    - Run test publisher

set -e

CONNECTOR=$1
COMMAND=${2:-start}

if [ -z "$CONNECTOR" ]; then
    echo "Usage: $0 <connector-name> [start|stop|logs|restart|test]"
    echo ""
    echo "Available connectors:"
    ls -1 examples/ 2>/dev/null | sed 's/^/  - /'
    exit 1
fi

EXAMPLE_DIR="examples/$CONNECTOR"

if [ ! -d "$EXAMPLE_DIR" ]; then
    echo "Error: Example not found: $EXAMPLE_DIR"
    echo ""
    echo "Available connectors:"
    ls -1 examples/ 2>/dev/null | sed 's/^/  - /'
    exit 1
fi

cd "$EXAMPLE_DIR"

case $COMMAND in
    start)
        echo "Starting $CONNECTOR example..."
        docker-compose up -d
        echo ""
        echo "✓ Example started!"
        echo ""
        echo "Useful commands:"
        echo "  View logs:    docker-compose logs -f"
        echo "  Stop:         $0 $CONNECTOR stop"
        echo "  Run tests:    $0 $CONNECTOR test"
        ;;
    
    stop)
        echo "Stopping $CONNECTOR example..."
        docker-compose down -v
        echo "✓ Example stopped!"
        ;;
    
    logs)
        CONTAINER_NAME="${CONNECTOR}-connector"
        if docker ps --format '{{.Names}}' | grep -q "$CONTAINER_NAME"; then
            docker-compose logs -f
        else
            echo "Error: Connector not running. Start it first:"
            echo "  $0 $CONNECTOR start"
            exit 1
        fi
        ;;
    
    restart)
        echo "Restarting $CONNECTOR example..."
        docker-compose restart
        echo "✓ Example restarted!"
        ;;
    
    test)
        if [ -f "test-publisher.sh" ]; then
            echo "Running test publisher for $CONNECTOR..."
            ./test-publisher.sh
        else
            echo "No test publisher found for $CONNECTOR"
            echo "You can publish messages manually - see README.md"
        fi
        ;;
    
    *)
        echo "Unknown command: $COMMAND"
        echo "Usage: $0 <connector-name> [start|stop|logs|restart|test]"
        exit 1
        ;;
esac
